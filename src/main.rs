mod app;
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
    // Discover available services
    let configs = discover_services(&working_dir)?;
    if configs.is_empty() {
        eprintln!("No Laravel services found in this directory");
        return Ok(());
    }

    // Create event channel
    let (event_tx, mut event_rx) = mpsc::channel::<Event>(100);

    // Create cancellation token for graceful shutdown
    let cancel_token = CancellationToken::new();

    // Initialize app state
    let mut app = App::new(working_dir.clone());
    for config in &configs {
        app.register_process(config.clone());
    }

    // Initialize process manager
    let mut process_manager = ProcessManager::new(event_tx.clone(), cancel_token.clone());
    for config in configs {
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
    for kind in app.process_order.clone() {
        if process_manager.is_running(kind) {
            app.set_process_status(kind, ProcessStatus::Running);
            app.set_process_pid(kind, process_manager.get_pid(kind));
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
                            KeyCode::Char('q') => {
                                // Restart queue worker
                                app.set_status("Restarting queue worker...");
                                app.set_process_status(
                                    process::ProcessKind::Queue,
                                    ProcessStatus::Restarting,
                                );
                                let _ = process_manager.restart(process::ProcessKind::Queue).await;
                                app.set_process_status(
                                    process::ProcessKind::Queue,
                                    ProcessStatus::Running,
                                );
                                app.clear_status();
                            }
                            KeyCode::Char('v') => {
                                // Restart Vite
                                app.set_status("Restarting Vite...");
                                app.set_process_status(
                                    process::ProcessKind::Vite,
                                    ProcessStatus::Restarting,
                                );
                                let _ = process_manager.restart(process::ProcessKind::Vite).await;
                                app.set_process_status(
                                    process::ProcessKind::Vite,
                                    ProcessStatus::Running,
                                );
                                app.clear_status();
                            }
                            KeyCode::Char('s') => {
                                // Restart Serve
                                app.set_status("Restarting serve...");
                                app.set_process_status(
                                    process::ProcessKind::Serve,
                                    ProcessStatus::Restarting,
                                );
                                let _ = process_manager.restart(process::ProcessKind::Serve).await;
                                app.set_process_status(
                                    process::ProcessKind::Serve,
                                    ProcessStatus::Running,
                                );
                                app.clear_status();
                            }
                            KeyCode::Char('b') => {
                                // Restart Reverb
                                app.set_status("Restarting Reverb...");
                                app.set_process_status(
                                    process::ProcessKind::Reverb,
                                    ProcessStatus::Restarting,
                                );
                                let _ = process_manager.restart(process::ProcessKind::Reverb).await;
                                app.set_process_status(
                                    process::ProcessKind::Reverb,
                                    ProcessStatus::Running,
                                );
                                app.clear_status();
                            }
                            KeyCode::Char('h') => {
                                // Restart Horizon
                                app.set_status("Restarting Horizon...");
                                app.set_process_status(
                                    process::ProcessKind::Horizon,
                                    ProcessStatus::Restarting,
                                );
                                let _ = process_manager.restart(process::ProcessKind::Horizon).await;
                                app.set_process_status(
                                    process::ProcessKind::Horizon,
                                    ProcessStatus::Running,
                                );
                                app.clear_status();
                            }
                            KeyCode::Char('r') => {
                                // Restart all
                                app.set_status("Restarting all processes...");
                                for kind in app.process_order.clone() {
                                    app.set_process_status(kind, ProcessStatus::Restarting);
                                }
                                let _ = process_manager.restart_all().await;
                                for kind in app.process_order.clone() {
                                    app.set_process_status(kind, ProcessStatus::Running);
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
                            _ => {}
                        }
                    }
                }
                Event::ProcessOutput {
                    kind,
                    line,
                    is_stderr,
                } => {
                    app.add_process_output(kind, line, is_stderr);
                }
                Event::ProcessExited { kind, exit_code } => {
                    let status = if exit_code == Some(0) {
                        ProcessStatus::Stopped
                    } else {
                        ProcessStatus::Failed
                    };
                    app.set_process_status(kind, status);
                    app.set_process_pid(kind, None);
                }
                Event::LogUpdate(lines) => {
                    app.add_log_lines(lines);
                }
                Event::Resize(_, _) => {
                    // Terminal will handle resize on next draw
                }
                Event::Tick => {
                    // Update process status from manager
                    for kind in app.process_order.clone() {
                        let is_running = process_manager.is_running(kind);
                        let current_status = app
                            .processes
                            .get(&kind)
                            .map(|p| p.status)
                            .unwrap_or(ProcessStatus::Stopped);

                        // Only update if not in transitional state
                        if !matches!(current_status, ProcessStatus::Restarting)
                            && is_running
                            && current_status != ProcessStatus::Running
                        {
                            app.set_process_status(kind, ProcessStatus::Running);
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
