mod app;
mod config;
mod error;
mod event;
mod log;
mod process;
mod tui;
mod ui;

use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use app::App;
use config::LaramuxConfig;
use error::Result;
use event::Event;
use log::{find_log_file, LogWatcher};
use process::{discover_services, ProcessManager, ProcessStatus};

const TICK_RATE: Duration = Duration::from_millis(100);

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook for terminal restoration
    tui::install_panic_hook();

    // Get working directory
    let working_dir = std::env::current_dir()?;

    // Run the application
    run(working_dir).await
}

async fn run(working_dir: PathBuf) -> Result<()> {
    // Load configuration (optional)
    let config = match LaramuxConfig::load(&working_dir) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Failed to load .laramux.json: {}", e);
            None
        }
    };

    // Discover available services
    let discovery_result = discover_services(&working_dir, config.as_ref())?;
    if discovery_result.configs.is_empty() {
        eprintln!("No Laravel services found in this directory");
        return Ok(());
    }

    // Create event channel
    let (event_tx, mut event_rx) = mpsc::channel::<Event>(100);

    // Create cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

    // Initialize app state
    let mut app = App::new(working_dir.clone());
    app.set_registry(discovery_result.registry);
    for config in &discovery_result.configs {
        app.register_process(config.clone());
    }

    // Initialize process manager
    let mut process_manager = ProcessManager::new(event_tx.clone(), cancel_token.clone());
    for config in discovery_result.configs {
        process_manager.register(config);
    }

    // Initialize terminal
    let mut terminal = tui::init()?;

    // Spawn input handler task
    let input_tx = event_tx.clone();
    let input_token = cancel_token.clone();
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        loop {
            tokio::select! {
                _ = input_token.cancelled() => break,
                Some(Ok(event)) = reader.next() => {
                    match event {
                        CrosstermEvent::Key(key) => {
                            let _ = input_tx.send(Event::Input(key)).await;
                        }
                        CrosstermEvent::Resize(w, h) => {
                            let _ = input_tx.send(Event::Resize(w, h)).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Spawn tick task
    let tick_tx = event_tx.clone();
    let tick_token = cancel_token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK_RATE);
        loop {
            tokio::select! {
                _ = tick_token.cancelled() => break,
                _ = interval.tick() => {
                    let _ = tick_tx.send(Event::Tick).await;
                }
            }
        }
    });

    // Spawn log watcher if log file exists
    if let Some(log_path) = find_log_file(&working_dir) {
        let watcher = LogWatcher::new(log_path, event_tx.clone(), cancel_token.clone());
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });
    }

    // Start all processes
    process_manager.spawn_all().await?;

    // Update initial status
    for id in app.process_order.clone() {
        if process_manager.is_running(&id) {
            app.set_process_status(&id, ProcessStatus::Running);
            app.set_process_pid(&id, process_manager.get_pid(&id));
        }
    }

    // Main event loop
    loop {
        // Render UI
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Handle events
        if let Some(event) = event_rx.recv().await {
            match event {
                Event::Input(key) => {
                    // Handle Ctrl+C
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        app.quit();
                    } else {
                        match key.code {
                            KeyCode::Up => app.select_previous(),
                            KeyCode::Down => app.select_next(),
                            KeyCode::Char('r') => {
                                // Restart all
                                app.set_status("Restarting all processes...");
                                for id in app.process_order.clone() {
                                    app.set_process_status(&id, ProcessStatus::Restarting);
                                }
                                let _ = process_manager.restart_all().await;
                                for id in app.process_order.clone() {
                                    app.set_process_status(&id, ProcessStatus::Running);
                                }
                                app.clear_status();
                            }
                            KeyCode::Char('c') => {
                                // Clear output
                                app.clear_selected_output();
                            }
                            KeyCode::PageUp => {
                                app.scroll_output_up(10);
                            }
                            KeyCode::PageDown => {
                                app.scroll_output_down(10);
                            }
                            KeyCode::Char(ch) => {
                                // Dynamic hotkey handling via registry
                                if let Some(id) =
                                    app.registry.find_by_hotkey(ch, &app.process_order)
                                {
                                    let display_name = app.registry.display_name(&id);
                                    app.set_status(format!("Restarting {}...", display_name));
                                    app.set_process_status(&id, ProcessStatus::Restarting);
                                    let _ = process_manager.restart(&id).await;
                                    app.set_process_status(&id, ProcessStatus::Running);
                                    app.clear_status();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::ProcessOutput {
                    id,
                    line,
                    is_stderr,
                } => {
                    app.add_process_output(&id, line, is_stderr);
                }
                Event::ProcessExited { id, exit_code } => {
                    let status = if exit_code == Some(0) {
                        ProcessStatus::Stopped
                    } else {
                        ProcessStatus::Failed
                    };
                    app.set_process_status(&id, status);
                    app.set_process_pid(&id, None);
                }
                Event::LogUpdate(lines) => {
                    app.add_log_lines(lines);
                }
                Event::Resize(_, _) => {
                    // Terminal will handle resize on next draw
                }
                Event::Tick => {
                    // Update process status from manager
                    for id in app.process_order.clone() {
                        let is_running = process_manager.is_running(&id);
                        let current_status = app
                            .processes
                            .get(&id)
                            .map(|p| p.status)
                            .unwrap_or(ProcessStatus::Stopped);

                        // Only update if not in transitional state
                        if !matches!(current_status, ProcessStatus::Restarting)
                            && is_running
                            && current_status != ProcessStatus::Running
                        {
                            app.set_process_status(&id, ProcessStatus::Running);
                        }
                    }
                }
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    // Cleanup
    cancel_token.cancel();
    process_manager.kill_all().await?;
    tui::restore()?;

    Ok(())
}
