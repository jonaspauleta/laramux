mod app;
mod config;
mod error;
mod event;
mod log;
mod process;
mod tui;
mod ui;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use sysinfo::System;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

/// Sender for writing to a running command's stdin
type CommandStdinWriter = Arc<Mutex<Option<tokio::process::ChildStdin>>>;

use app::{App, ProcessesView};
use config::LaramuxConfig;
use error::Result;
use event::Event;
use log::{find_log_dir, LogWatcher};
use process::types::OutputLine;
use process::{discover_services, ProcessManager, ProcessStatus};
use ui::tabs::Tab;

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
    let (config, config_error) = match LaramuxConfig::load(&working_dir) {
        Ok(cfg) => (cfg, None),
        Err(e) => {
            // Format a user-friendly error message
            let error_msg = format_config_error(&e);
            (None, Some(error_msg))
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
    app.is_sail = discovery_result.is_sail;
    app.set_config(config);
    if let Some(error) = config_error {
        app.set_config_error(error);
    }
    if discovery_result.is_sail {
        if discovery_result.supervised_kinds.is_empty() {
            app.set_status("Laravel Sail detected — running in Docker mode");
        } else {
            let names: Vec<&str> = discovery_result
                .supervised_kinds
                .iter()
                .map(|k| k.display_name())
                .collect();
            app.set_status(format!(
                "Laravel Sail detected — {} supervised",
                names.join(", ")
            ));
        }
    }
    app.set_registry(discovery_result.registry);
    app.set_artisan_commands(discovery_result.artisan_commands);
    app.set_artisan_make_commands(discovery_result.artisan_make_commands);
    app.set_quality_tools(discovery_result.quality_tools);
    app.set_testing_tools(discovery_result.testing_tools);
    for config in &discovery_result.configs {
        app.register_process(config.clone());
    }

    // Initialize process manager
    let mut process_manager = ProcessManager::new(event_tx.clone(), cancel_token.clone());
    for config in discovery_result.configs {
        process_manager.register(config);
    }

    // Command runner cancellation token (for cancelling running commands)
    let command_cancel = Arc::new(Mutex::new(None::<CancellationToken>));
    // Command stdin writer (for sending input to running commands)
    let command_stdin_writer: CommandStdinWriter = Arc::new(Mutex::new(None));

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
                        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
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

    // Spawn system stats monitoring task
    let stats_tx = event_tx.clone();
    let stats_token = cancel_token.clone();
    tokio::spawn(async move {
        let mut sys = System::new_all();
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = stats_token.cancelled() => break,
                _ = interval.tick() => {
                    sys.refresh_all();

                    let total_memory = sys.total_memory();
                    let used_memory = sys.used_memory();
                    let memory_usage = if total_memory > 0 {
                        (used_memory as f32 / total_memory as f32) * 100.0
                    } else {
                        0.0
                    };

                    let cpu_usage = sys.global_cpu_usage();

                    let mut process_stats = std::collections::HashMap::new();
                    for (pid, process) in sys.processes() {
                        process_stats.insert(
                            pid.as_u32(),
                            app::ProcessStats {
                                cpu_usage: process.cpu_usage(),
                                memory_bytes: process.memory(),
                            },
                        );
                    }

                    let stats = app::SystemStats {
                        cpu_usage,
                        memory_usage,
                        total_memory,
                        used_memory,
                        process_stats,
                    };

                    let _ = stats_tx.send(Event::SystemStatsUpdate(stats)).await;
                }
            }
        }
    });

    // Spawn log watcher if log directory exists
    if let Some(log_dir) = find_log_dir(&working_dir) {
        // Get additional log files from config
        let additional_files: Vec<std::path::PathBuf> = app
            .config
            .as_ref()
            .map(|c| {
                c.additional_log_files()
                    .iter()
                    .map(|f| working_dir.join(f))
                    .collect()
            })
            .unwrap_or_default();

        let watcher = LogWatcher::new(log_dir, event_tx.clone(), cancel_token.clone())
            .with_additional_files(additional_files);
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });
    }

    // Start all processes
    let spawn_errors = process_manager.spawn_all().await?;

    // Update initial status
    for id in app.process_order.clone() {
        if process_manager.is_running(&id) {
            if process_manager.is_supervised(&id) {
                app.set_process_status(&id, ProcessStatus::Supervised);
            } else {
                app.set_process_status(&id, ProcessStatus::Running);
            }
            app.set_process_pid(&id, process_manager.get_pid(&id));
        } else {
            // Check if this process had a spawn error
            if let Some((_, error)) = spawn_errors.iter().find(|(err_id, _)| err_id == &id) {
                app.set_process_status(&id, ProcessStatus::Failed);
                // Add error to process output
                app.add_process_output(&id, error.clone(), true);
            }
        }
    }

    // Show a status message if there were spawn errors
    if !spawn_errors.is_empty() {
        let failed_names: Vec<_> = spawn_errors.iter().map(|(id, _)| id.to_string()).collect();
        app.set_status(format!(
            "Warning: Failed to start {} process(es): {}",
            spawn_errors.len(),
            failed_names.join(", ")
        ));
    }

    // Main event loop
    loop {
        // Render UI
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Handle events
        if let Some(event) = event_rx.recv().await {
            match event {
                Event::Input(key) => {
                    // Handle Ctrl+C globally
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        app.set_status("Quitting...");
                        app.quit();
                        // Render one more time to show the quit message
                        terminal.draw(|frame| ui::render(frame, &app))?;
                        break;
                    }

                    // Check for input modes first (search, command args)
                    if handle_input_mode(&mut app, &key) {
                        continue;
                    }

                    // Handle global tab navigation
                    if handle_global_keys(&mut app, &key) {
                        continue;
                    }

                    // Handle tab-specific keys
                    handle_tab_keys(
                        &mut app,
                        &key,
                        &mut process_manager,
                        &event_tx,
                        &command_cancel,
                        &command_stdin_writer,
                        &working_dir,
                        &cancel_token,
                    )
                    .await;
                }
                Event::ProcessOutput {
                    id,
                    line,
                    is_stderr,
                } => {
                    app.add_process_output(&id, line, is_stderr);
                }
                Event::ProcessExited { id, exit_code } => {
                    let status = if process_manager.is_supervised(&id) {
                        // Supervised: log tail ending is normal, always Stopped
                        ProcessStatus::Stopped
                    } else if exit_code == Some(0) {
                        ProcessStatus::Stopped
                    } else {
                        ProcessStatus::Failed
                    };
                    app.set_process_status(&id, status);
                    app.set_process_pid(&id, None);

                    // Check for auto-restart based on restart policy
                    if process_manager.should_restart(&id, exit_code) {
                        // Record failure for backoff calculation
                        if exit_code != Some(0) {
                            process_manager.record_failure(&id);
                        }

                        let backoff = process_manager.get_backoff_delay(&id);
                        let display_name = app.registry.display_name(&id);
                        app.set_status(format!(
                            "Auto-restarting {} in {:.1}s...",
                            display_name,
                            backoff.as_secs_f32()
                        ));
                        app.set_process_status(&id, ProcessStatus::Restarting);

                        // Spawn delayed restart task
                        let restart_id = id.clone();
                        let restart_tx = event_tx.clone();
                        let restart_token = cancel_token.clone();
                        tokio::spawn(async move {
                            tokio::select! {
                                _ = restart_token.cancelled() => {}
                                _ = tokio::time::sleep(backoff) => {
                                    // Signal main loop to restart the process
                                    let _ = restart_tx.send(Event::ProcessAutoRestart {
                                        id: restart_id,
                                    }).await;
                                }
                            }
                        });
                    }
                }
                Event::ProcessAutoRestart { id } => {
                    // Handle auto-restart request
                    let display_name = app.registry.display_name(&id);
                    app.set_status(format!("Restarting {}...", display_name));
                    if let Err(e) = process_manager.spawn(&id).await {
                        app.set_status(format!("Failed to restart {}: {}", display_name, e));
                        app.set_process_status(&id, ProcessStatus::Failed);
                    } else {
                        if process_manager.is_supervised(&id) {
                            app.set_process_status(&id, ProcessStatus::Supervised);
                        } else {
                            app.set_process_status(&id, ProcessStatus::Running);
                        }
                        app.set_process_pid(&id, process_manager.get_pid(&id));
                        app.clear_status();
                    }
                }
                Event::LogUpdate(lines) => {
                    app.add_log_lines(lines);
                }
                Event::CommandOutput { line, is_stderr } => {
                    let output_line = if is_stderr {
                        OutputLine::stderr(line)
                    } else {
                        OutputLine::stdout(line)
                    };
                    // Send to whichever tab has a running command
                    if app.artisan_tab.running_command.is_some() {
                        app.artisan_tab.add_output(output_line);
                    } else if app.make_tab.running_command.is_some() {
                        app.make_tab.add_output(output_line);
                    } else if app.quality_tab.running_command.is_some() {
                        app.quality_tab.add_output(output_line);
                    }
                }
                Event::CommandExited { exit_code } => {
                    let msg = match exit_code {
                        Some(0) => "Command completed successfully".to_string(),
                        Some(code) => format!("Command exited with code {}", code),
                        None => "Command terminated".to_string(),
                    };
                    // Clear running command on whichever tab has it
                    if app.artisan_tab.running_command.is_some() {
                        app.artisan_tab.running_command = None;
                        app.artisan_tab.input_mode = false;
                        app.artisan_tab
                            .add_output(OutputLine::stdout(format!("\n--- {} ---", msg)));
                    } else if app.make_tab.running_command.is_some() {
                        app.make_tab.running_command = None;
                        app.make_tab.input_mode = false;
                        app.make_tab
                            .add_output(OutputLine::stdout(format!("\n--- {} ---", msg)));
                    } else if app.quality_tab.running_command.is_some() {
                        app.quality_tab.running_command = None;
                        app.quality_tab.input_mode = false;
                        app.quality_tab
                            .add_output(OutputLine::stdout(format!("\n--- {} ---", msg)));
                    }
                    // Clear stdin sender
                    let mut guard = command_stdin_writer.lock().await;
                    *guard = None;
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

                        if !matches!(current_status, ProcessStatus::Restarting) && is_running {
                            let expected = if process_manager.is_supervised(&id) {
                                ProcessStatus::Supervised
                            } else {
                                ProcessStatus::Running
                            };
                            if current_status != expected {
                                app.set_process_status(&id, expected);
                            }
                        }
                    }
                }
                Event::SystemStatsUpdate(stats) => {
                    app.system_stats = stats;
                }
            }
        }

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

/// Handle input mode keys (search input, command args input)
/// Returns true if the key was handled
fn handle_input_mode(app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
    // Logs tab search input mode
    if app.active_tab == Tab::Logs && app.logs_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.logs_tab.input_mode = false;
            }
            KeyCode::Enter => {
                app.logs_tab.input_mode = false;
            }
            KeyCode::Backspace => {
                app.logs_tab.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.logs_tab.search_query.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Artisan tab input mode
    if app.active_tab == Tab::Artisan && app.artisan_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.artisan_tab.input_mode = false;
            }
            KeyCode::Enter => {
                app.artisan_tab.input_mode = false;
            }
            KeyCode::Backspace => {
                app.artisan_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.artisan_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Artisan tab search mode
    if app.active_tab == Tab::Artisan && app.artisan_tab.search_mode {
        match key.code {
            KeyCode::Esc => {
                app.artisan_tab.search_mode = false;
            }
            KeyCode::Enter => {
                app.artisan_tab.search_mode = false;
            }
            KeyCode::Backspace => {
                app.artisan_tab.search_query.pop();
                app.artisan_tab.selected_command = 0;
            }
            KeyCode::Char(c) => {
                app.artisan_tab.search_query.push(c);
                app.artisan_tab.selected_command = 0;
            }
            _ => {}
        }
        return true;
    }

    // Make tab input mode
    if app.active_tab == Tab::Make && app.make_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.make_tab.input_mode = false;
            }
            KeyCode::Enter => {
                app.make_tab.input_mode = false;
            }
            KeyCode::Backspace => {
                app.make_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.make_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return true;
    }

    // Make tab search mode
    if app.active_tab == Tab::Make && app.make_tab.search_mode {
        match key.code {
            KeyCode::Esc => {
                app.make_tab.search_mode = false;
            }
            KeyCode::Enter => {
                app.make_tab.search_mode = false;
            }
            KeyCode::Backspace => {
                app.make_tab.search_query.pop();
                app.make_tab.selected_command = 0;
            }
            KeyCode::Char(c) => {
                app.make_tab.search_query.push(c);
                app.make_tab.selected_command = 0;
            }
            _ => {}
        }
        return true;
    }

    // Quality tab input mode
    if app.active_tab == Tab::Quality && app.quality_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.quality_tab.input_mode = false;
            }
            KeyCode::Enter => {
                app.quality_tab.input_mode = false;
            }
            KeyCode::Backspace => {
                app.quality_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.quality_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return true;
    }

    false
}

/// Handle global navigation keys
/// Returns true if the key was handled
fn handle_global_keys(app: &mut App, key: &crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('1') => {
            app.go_to_tab(Tab::Processes);
            true
        }
        KeyCode::Char('2') => {
            app.go_to_tab(Tab::Logs);
            true
        }
        KeyCode::Char('3') => {
            app.go_to_tab(Tab::Artisan);
            true
        }
        KeyCode::Char('4') => {
            app.go_to_tab(Tab::Make);
            true
        }
        KeyCode::Char('5') => {
            app.go_to_tab(Tab::Quality);
            true
        }
        KeyCode::Char('6') => {
            app.go_to_tab(Tab::Config);
            true
        }
        KeyCode::Char('?') => {
            app.go_to_tab(Tab::About);
            true
        }
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.previous_tab();
            } else {
                app.next_tab();
            }
            true
        }
        KeyCode::BackTab => {
            app.previous_tab();
            true
        }
        _ => false,
    }
}

/// Handle tab-specific keys
#[allow(clippy::too_many_arguments)]
async fn handle_tab_keys(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    process_manager: &mut ProcessManager,
    event_tx: &mpsc::Sender<Event>,
    command_cancel: &Arc<Mutex<Option<CancellationToken>>>,
    command_stdin_writer: &CommandStdinWriter,
    working_dir: &Path,
    main_cancel: &CancellationToken,
) {
    match app.active_tab {
        Tab::Processes => {
            handle_processes_keys(app, key, process_manager).await;
        }
        Tab::Logs => {
            handle_logs_keys(app, key);
        }
        Tab::Artisan => {
            handle_artisan_keys(
                app,
                key,
                event_tx,
                command_cancel,
                command_stdin_writer,
                working_dir,
                main_cancel,
            )
            .await;
        }
        Tab::Make => {
            handle_make_keys(
                app,
                key,
                event_tx,
                command_cancel,
                command_stdin_writer,
                working_dir,
                main_cancel,
            )
            .await;
        }
        Tab::Quality => {
            handle_quality_keys(
                app,
                key,
                event_tx,
                command_cancel,
                command_stdin_writer,
                working_dir,
                main_cancel,
            )
            .await;
        }
        Tab::Config => {
            handle_config_keys(app, key, working_dir);
        }
        Tab::About => {
            // About tab has no special keys
        }
    }
}

async fn handle_processes_keys(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    process_manager: &mut ProcessManager,
) {
    match app.processes_tab.view {
        ProcessesView::List => {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_previous();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
                }
                KeyCode::Enter => {
                    app.processes_tab.toggle_view();
                }
                KeyCode::Char('s') => {
                    // Start selected process (or tail logs for supervised)
                    if let Some(id) = app.selected_id().cloned() {
                        let display_name = app.registry.display_name(&id);
                        let is_sup = process_manager.is_supervised(&id);
                        if is_sup {
                            app.set_status(format!("Tailing logs for {}...", display_name));
                        } else {
                            app.set_status(format!("Starting {}...", display_name));
                        }
                        let _ = process_manager.spawn(&id).await;
                        if is_sup {
                            app.set_process_status(&id, ProcessStatus::Supervised);
                        } else {
                            app.set_process_status(&id, ProcessStatus::Running);
                        }
                        app.set_process_pid(&id, process_manager.get_pid(&id));
                        app.clear_status();
                    }
                }
                KeyCode::Char('x') => {
                    // Stop selected process (or detach for supervised)
                    if let Some(id) = app.selected_id().cloned() {
                        let display_name = app.registry.display_name(&id);
                        if process_manager.is_supervised(&id) {
                            app.set_status(format!("Detaching from {}...", display_name));
                        } else {
                            app.set_status(format!("Stopping {}...", display_name));
                        }
                        let _ = process_manager.kill(&id).await;
                        app.set_process_status(&id, ProcessStatus::Stopped);
                        app.clear_status();
                    }
                }
                KeyCode::Char('r') => {
                    // Restart selected process (or reconnect for supervised)
                    if let Some(id) = app.selected_id().cloned() {
                        let display_name = app.registry.display_name(&id);
                        if process_manager.is_supervised(&id) {
                            app.set_status(format!("Reconnecting to {}...", display_name));
                        } else {
                            app.set_status(format!("Restarting {}...", display_name));
                        }
                        app.set_process_status(&id, ProcessStatus::Restarting);
                        let _ = process_manager.restart(&id).await;
                        if process_manager.is_supervised(&id) {
                            app.set_process_status(&id, ProcessStatus::Supervised);
                        }
                        app.clear_status();
                    }
                }
                KeyCode::Char('R') => {
                    // Restart all
                    app.set_status("Restarting all processes...");
                    for id in app.process_order.clone() {
                        app.set_process_status(&id, ProcessStatus::Restarting);
                    }
                    let _ = process_manager.restart_all().await;
                    // Status will be updated by tick event when processes are running
                    app.clear_status();
                }
                KeyCode::Char(ch) => {
                    // Dynamic hotkey handling via registry (s/v/q/h/b)
                    if let Some(id) = app.registry.find_by_hotkey(ch, &app.process_order) {
                        let display_name = app.registry.display_name(&id);
                        app.set_status(format!("Restarting {}...", display_name));
                        app.set_process_status(&id, ProcessStatus::Restarting);
                        let _ = process_manager.restart(&id).await;
                        // Status will be updated by tick event when process is running
                        app.clear_status();
                    }
                }
                _ => {}
            }
        }
        ProcessesView::Output => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.processes_tab.toggle_view();
            }
            KeyCode::Char('c') => {
                app.clear_selected_output();
            }
            KeyCode::Char('r') => {
                if let Some(id) = app.selected_id().cloned() {
                    let display_name = app.registry.display_name(&id);
                    app.set_status(format!("Restarting {}...", display_name));
                    app.set_process_status(&id, ProcessStatus::Restarting);
                    let _ = process_manager.restart(&id).await;
                    // Status will be updated by tick event when process is running
                    app.clear_status();
                }
            }
            KeyCode::PageUp => {
                app.scroll_output_up(10);
            }
            KeyCode::PageDown => {
                app.scroll_output_down(10);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.scroll_output_up(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.scroll_output_down(1);
            }
            _ => {}
        },
    }
}

fn handle_logs_keys(app: &mut App, key: &crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('/') => {
            app.logs_tab.input_mode = true;
        }
        KeyCode::Char('f') => {
            app.logs_tab.cycle_filter();
        }
        KeyCode::Char('F') => {
            app.logs_tab.cycle_file();
        }
        KeyCode::Char('c') => {
            app.clear_logs();
        }
        KeyCode::Char('g') => {
            // Go to top
            let filtered = app.filtered_logs();
            app.logs_tab.scroll_offset = filtered.len();
        }
        KeyCode::Char('G') => {
            // Go to bottom
            app.logs_tab.scroll_offset = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_log_up(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_log_down(1);
        }
        KeyCode::PageUp => {
            app.scroll_log_up(10);
        }
        KeyCode::PageDown => {
            app.scroll_log_down(10);
        }
        _ => {}
    }
}

async fn handle_artisan_keys(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    event_tx: &mpsc::Sender<Event>,
    command_cancel: &Arc<Mutex<Option<CancellationToken>>>,
    command_stdin_writer: &CommandStdinWriter,
    working_dir: &Path,
    main_cancel: &CancellationToken,
) {
    // If a command is running and input mode is active, collect input in buffer
    if app.artisan_tab.running_command.is_some() && app.artisan_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.artisan_tab.input_mode = false;
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut app.artisan_tab.input_buffer);
                let mut guard = command_stdin_writer.lock().await;
                if let Some(ref mut stdin) = *guard {
                    let line = format!("{}\n", input);
                    let _ = stdin.write_all(line.as_bytes()).await;
                    let _ = stdin.flush().await;
                }
            }
            KeyCode::Backspace => {
                app.artisan_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.artisan_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    // Get favorites for various operations
    let favorites: Vec<String> = app
        .config
        .as_ref()
        .map(|c| c.artisan_favorites().to_vec())
        .unwrap_or_default();

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.artisan_tab.details_scroll_offset > 0 {
                    app.artisan_tab.details_scroll_offset -= 1;
                }
            } else if app.artisan_tab.selected_command > 0 {
                app.artisan_tab.selected_command -= 1;
                app.artisan_tab.details_scroll_offset = 0;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.artisan_tab.details_scroll_offset += 1;
            } else {
                let max = app.artisan_tab.command_count(&favorites).saturating_sub(1);
                if app.artisan_tab.selected_command < max {
                    app.artisan_tab.selected_command += 1;
                    app.artisan_tab.details_scroll_offset = 0;
                }
            }
        }
        KeyCode::Char('/') => {
            app.artisan_tab.search_mode = true;
        }
        KeyCode::Char('i') => {
            app.artisan_tab.input_mode = true;
        }
        KeyCode::Char('c') => {
            app.artisan_tab.clear_output();
        }
        KeyCode::Char('f') => {
            // Toggle favorite for the selected command
            if let Some(cmd_name) = app.artisan_tab.selected_command_name(&favorites) {
                // Initialize config if it doesn't exist
                if app.config.is_none() {
                    app.config = Some(LaramuxConfig::default());
                }
                if let Some(ref mut config) = app.config {
                    config.toggle_artisan_favorite(&cmd_name);
                    if let Err(e) = config.save(working_dir) {
                        app.set_status(format!("Failed to save config: {}", e));
                    }
                }
            }
        }
        KeyCode::PageUp => {
            app.artisan_tab.output_scroll_offset =
                app.artisan_tab.output_scroll_offset.saturating_add(10);
        }
        KeyCode::PageDown => {
            app.artisan_tab.output_scroll_offset =
                app.artisan_tab.output_scroll_offset.saturating_sub(10);
        }
        KeyCode::Enter => {
            if app.artisan_tab.running_command.is_some() {
                app.set_status("A command is already running");
                return;
            }

            let user_args = app.artisan_tab.input_buffer.clone();
            if let Some(resolved) =
                app.artisan_tab
                    .selected_command_resolved(&user_args, &favorites, app.is_sail)
            {
                spawn_command(
                    app,
                    resolved,
                    event_tx,
                    command_cancel,
                    command_stdin_writer,
                    working_dir,
                    main_cancel,
                    CommandTab::Artisan,
                )
                .await;
            }
        }
        KeyCode::Esc => {
            let guard = command_cancel.lock().await;
            if let Some(ref cancel) = *guard {
                cancel.cancel();
            }
        }
        _ => {}
    }
}

async fn handle_make_keys(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    event_tx: &mpsc::Sender<Event>,
    command_cancel: &Arc<Mutex<Option<CancellationToken>>>,
    command_stdin_writer: &CommandStdinWriter,
    working_dir: &Path,
    main_cancel: &CancellationToken,
) {
    // If a command is running and input mode is active, collect input in buffer
    if app.make_tab.running_command.is_some() && app.make_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.make_tab.input_mode = false;
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut app.make_tab.input_buffer);
                let mut guard = command_stdin_writer.lock().await;
                if let Some(ref mut stdin) = *guard {
                    let line = format!("{}\n", input);
                    let _ = stdin.write_all(line.as_bytes()).await;
                    let _ = stdin.flush().await;
                }
            }
            KeyCode::Backspace => {
                app.make_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.make_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    // Get favorites for various operations
    let favorites: Vec<String> = app
        .config
        .as_ref()
        .map(|c| c.make_favorites().to_vec())
        .unwrap_or_default();

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.make_tab.details_scroll_offset > 0 {
                    app.make_tab.details_scroll_offset -= 1;
                }
            } else if app.make_tab.selected_command > 0 {
                app.make_tab.selected_command -= 1;
                app.make_tab.details_scroll_offset = 0;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.make_tab.details_scroll_offset += 1;
            } else {
                let max = app.make_tab.command_count(&favorites).saturating_sub(1);
                if app.make_tab.selected_command < max {
                    app.make_tab.selected_command += 1;
                    app.make_tab.details_scroll_offset = 0;
                }
            }
        }
        KeyCode::Char('/') => {
            app.make_tab.search_mode = true;
        }
        KeyCode::Char('i') => {
            app.make_tab.input_mode = true;
        }
        KeyCode::Char('c') => {
            app.make_tab.clear_output();
        }
        KeyCode::Char('f') => {
            // Toggle favorite for the selected command
            if let Some(cmd_name) = app.make_tab.selected_command_name(&favorites) {
                // Initialize config if it doesn't exist
                if app.config.is_none() {
                    app.config = Some(LaramuxConfig::default());
                }
                if let Some(ref mut config) = app.config {
                    config.toggle_make_favorite(&cmd_name);
                    if let Err(e) = config.save(working_dir) {
                        app.set_status(format!("Failed to save config: {}", e));
                    }
                }
            }
        }
        KeyCode::PageUp => {
            app.make_tab.output_scroll_offset =
                app.make_tab.output_scroll_offset.saturating_add(10);
        }
        KeyCode::PageDown => {
            app.make_tab.output_scroll_offset =
                app.make_tab.output_scroll_offset.saturating_sub(10);
        }
        KeyCode::Enter => {
            if app.make_tab.running_command.is_some() {
                app.set_status("A command is already running");
                return;
            }

            // Make commands require a name
            if app.make_tab.input_buffer.trim().is_empty() {
                app.set_status("Name required - press [i] to enter a name first");
                return;
            }

            let user_args = app.make_tab.input_buffer.clone();
            if let Some(resolved) =
                app.make_tab
                    .selected_command_resolved(&user_args, &favorites, app.is_sail)
            {
                spawn_command(
                    app,
                    resolved,
                    event_tx,
                    command_cancel,
                    command_stdin_writer,
                    working_dir,
                    main_cancel,
                    CommandTab::Make,
                )
                .await;
            }
        }
        KeyCode::Esc => {
            let guard = command_cancel.lock().await;
            if let Some(ref cancel) = *guard {
                cancel.cancel();
            }
        }
        _ => {}
    }
}

async fn handle_quality_keys(
    app: &mut App,
    key: &crossterm::event::KeyEvent,
    event_tx: &mpsc::Sender<Event>,
    command_cancel: &Arc<Mutex<Option<CancellationToken>>>,
    command_stdin_writer: &CommandStdinWriter,
    working_dir: &Path,
    main_cancel: &CancellationToken,
) {
    // If a command is running and input mode is active, collect input in buffer
    if app.quality_tab.running_command.is_some() && app.quality_tab.input_mode {
        match key.code {
            KeyCode::Esc => {
                app.quality_tab.input_mode = false;
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut app.quality_tab.input_buffer);
                let mut guard = command_stdin_writer.lock().await;
                if let Some(ref mut stdin) = *guard {
                    let line = format!("{}\n", input);
                    let _ = stdin.write_all(line.as_bytes()).await;
                    let _ = stdin.flush().await;
                }
            }
            KeyCode::Backspace => {
                app.quality_tab.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.quality_tab.input_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => {
            app.quality_tab.selected_category = app.quality_tab.selected_category.previous();
            app.quality_tab.selected_tool = 0;
            app.quality_tab.details_scroll_offset = 0;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.quality_tab.selected_category = app.quality_tab.selected_category.next();
            app.quality_tab.selected_tool = 0;
            app.quality_tab.details_scroll_offset = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.quality_tab.details_scroll_offset > 0 {
                    app.quality_tab.details_scroll_offset -= 1;
                }
            } else if app.quality_tab.selected_tool > 0 {
                app.quality_tab.selected_tool -= 1;
                app.quality_tab.details_scroll_offset = 0;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.quality_tab.details_scroll_offset += 1;
            } else {
                let max = app.quality_tab.tool_count().saturating_sub(1);
                if app.quality_tab.selected_tool < max {
                    app.quality_tab.selected_tool += 1;
                    app.quality_tab.details_scroll_offset = 0;
                }
            }
        }
        KeyCode::Char('i') => {
            app.quality_tab.input_mode = true;
        }
        KeyCode::Char('c') => {
            app.quality_tab.clear_output();
        }
        KeyCode::PageUp => {
            app.quality_tab.output_scroll_offset =
                app.quality_tab.output_scroll_offset.saturating_add(10);
        }
        KeyCode::PageDown => {
            app.quality_tab.output_scroll_offset =
                app.quality_tab.output_scroll_offset.saturating_sub(10);
        }
        KeyCode::Enter => {
            if app.quality_tab.running_command.is_some() {
                app.set_status("A command is already running");
                return;
            }

            let user_args = app.quality_tab.input_buffer.clone();
            if let Some(resolved) = app.quality_tab.selected_command_resolved(&user_args) {
                spawn_command(
                    app,
                    resolved,
                    event_tx,
                    command_cancel,
                    command_stdin_writer,
                    working_dir,
                    main_cancel,
                    CommandTab::Quality,
                )
                .await;
            }
        }
        KeyCode::Esc => {
            let guard = command_cancel.lock().await;
            if let Some(ref cancel) = *guard {
                cancel.cancel();
            }
        }
        _ => {}
    }
}

/// Which tab initiated the command
#[derive(Clone, Copy)]
enum CommandTab {
    Artisan,
    Make,
    Quality,
}

#[allow(clippy::too_many_arguments)]
async fn spawn_command(
    app: &mut App,
    resolved: app::ResolvedCommand,
    event_tx: &mpsc::Sender<Event>,
    command_cancel: &Arc<Mutex<Option<CancellationToken>>>,
    command_stdin_writer: &CommandStdinWriter,
    working_dir: &Path,
    main_cancel: &CancellationToken,
    tab: CommandTab,
) {
    let full_args = resolved.args.clone();

    let cmd_display = if full_args.is_empty() {
        resolved.command.clone()
    } else {
        format!("{} {}", resolved.command, full_args.join(" "))
    };

    // Set running command on the appropriate tab
    match tab {
        CommandTab::Artisan => {
            app.artisan_tab.running_command = Some(resolved.display_name.clone());
            app.artisan_tab
                .add_output(OutputLine::stdout(format!("$ {}", cmd_display)));
            app.artisan_tab.input_buffer.clear();
        }
        CommandTab::Make => {
            app.make_tab.running_command = Some(resolved.display_name.clone());
            app.make_tab
                .add_output(OutputLine::stdout(format!("$ {}", cmd_display)));
            app.make_tab.input_buffer.clear();
        }
        CommandTab::Quality => {
            app.quality_tab.running_command = Some(resolved.display_name.clone());
            app.quality_tab
                .add_output(OutputLine::stdout(format!("$ {}", cmd_display)));
            app.quality_tab.input_buffer.clear();
        }
    }

    // Spawn command runner
    let tx = event_tx.clone();
    let cmd = resolved.command.clone();
    let working_dir = working_dir.to_path_buf();
    let cancel = CancellationToken::new();
    let main_cancel = main_cancel.clone();
    let stdin_writer_clone = command_stdin_writer.clone();

    {
        let mut guard = command_cancel.lock().await;
        *guard = Some(cancel.clone());
    }

    tokio::spawn(async move {
        // Build and spawn command
        let mut child = match Command::new(&cmd)
            .args(&full_args)
            .current_dir(&working_dir)
            .env("FORCE_COLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = tx
                    .send(Event::CommandOutput {
                        line: format!("Failed to spawn command: {}", e),
                        is_stderr: true,
                    })
                    .await;
                let _ = tx.send(Event::CommandExited { exit_code: None }).await;
                return;
            }
        };

        // Take stdin for writing
        let stdin = child.stdin.take();
        {
            let mut guard = stdin_writer_clone.lock().await;
            *guard = stdin;
        }

        // Spawn stdout reader
        let stdout = child.stdout.take();
        let tx_stdout = tx.clone();
        let cancel_stdout = cancel.clone();
        if let Some(stdout) = stdout {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                loop {
                    tokio::select! {
                        _ = cancel_stdout.cancelled() => break,
                        result = reader.next_line() => {
                            match result {
                                Ok(Some(line)) => {
                                    let _ = tx_stdout
                                        .send(Event::CommandOutput {
                                            line,
                                            is_stderr: false,
                                        })
                                        .await;
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    }
                }
            });
        }

        // Spawn stderr reader
        let stderr = child.stderr.take();
        let tx_stderr = tx.clone();
        let cancel_stderr = cancel.clone();
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                loop {
                    tokio::select! {
                        _ = cancel_stderr.cancelled() => break,
                        result = reader.next_line() => {
                            match result {
                                Ok(Some(line)) => {
                                    let _ = tx_stderr
                                        .send(Event::CommandOutput {
                                            line,
                                            is_stderr: true,
                                        })
                                        .await;
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                    }
                }
            });
        }

        // Wait for process to exit
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                let _ = tx.send(Event::CommandExited { exit_code: None }).await;
            }
            _ = main_cancel.cancelled() => {
                let _ = child.kill().await;
            }
            result = child.wait() => {
                match result {
                    Ok(status) => {
                        let exit_code = status.code();
                        let _ = tx.send(Event::CommandExited { exit_code }).await;
                    }
                    Err(_) => {
                        let _ = tx.send(Event::CommandExited { exit_code: None }).await;
                    }
                }
            }
        }

        // Clear the stdin writer
        {
            let mut guard = stdin_writer_clone.lock().await;
            *guard = None;
        }
    });
}

fn handle_config_keys(app: &mut App, key: &crossterm::event::KeyEvent, working_dir: &Path) {
    use app::{ConfigDetailView, ConfigEditMode, ConfigFocus, ConfigSection};

    // Handle enum selection mode
    if app.config_tab.edit_mode == ConfigEditMode::SelectOption {
        // Compute the max index based on section
        let max_enum = match app.config_tab.section {
            ConfigSection::Sail => 2,               // 3 sail options
            ConfigSection::Logs => 8,               // 9 log level options
            ConfigSection::QualityCustomTools => 1, // 2 category options
            _ => 2,                                 // 3 restart policy options (default)
        };

        match key.code {
            KeyCode::Esc => {
                app.config_tab.edit_mode = ConfigEditMode::Browse;
                app.config_tab.enum_selection = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.config_tab.enum_selection > 0 {
                    app.config_tab.enum_selection -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.config_tab.enum_selection < max_enum {
                    app.config_tab.enum_selection += 1;
                }
            }
            KeyCode::Enter => {
                // Apply enum selection
                apply_enum_selection(app);
                app.config_tab.edit_mode = ConfigEditMode::Browse;
                app.config_tab.has_changes = true;
            }
            _ => {}
        }
        return;
    }

    // Handle edit mode input
    if app.config_tab.edit_mode == ConfigEditMode::EditText {
        match key.code {
            KeyCode::Esc => {
                app.config_tab.edit_mode = ConfigEditMode::Browse;
                app.config_tab.edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Apply edit
                apply_config_edit(app);
                app.config_tab.edit_mode = ConfigEditMode::Browse;
                app.config_tab.edit_buffer.clear();
                app.config_tab.has_changes = true;
            }
            KeyCode::Backspace => {
                app.config_tab.edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.config_tab.edit_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    // Handle delete confirmation
    if let Some(delete_idx) = app.config_tab.confirm_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // Perform delete
                perform_config_delete(app, delete_idx);
                app.config_tab.confirm_delete = None;
                app.config_tab.has_changes = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.config_tab.confirm_delete = None;
            }
            _ => {}
        }
        return;
    }

    // Clear error on any key press
    app.config_tab.error = None;

    // Check if we're in field view mode (navigating within an item's fields)
    let in_field_view = app.config_tab.detail_view == ConfigDetailView::ItemFields;

    match key.code {
        // Panel switching
        KeyCode::Left | KeyCode::Char('h') => {
            if in_field_view {
                // Exit field view back to item list
                app.config_tab.detail_view = ConfigDetailView::ItemList;
                app.config_tab.edit_field = 0;
            } else if app.config_tab.focus == ConfigFocus::Details {
                app.config_tab.focus = ConfigFocus::Sections;
                app.config_tab.selected_item = 0;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.config_tab.focus == ConfigFocus::Sections {
                app.config_tab.focus = ConfigFocus::Details;
                app.config_tab.selected_item = 0;
            } else if !in_field_view {
                // Enter field view for sections that support it
                match app.config_tab.section {
                    ConfigSection::Custom
                    | ConfigSection::QualityCustomTools
                    | ConfigSection::QualityDefaultArgs => {
                        app.config_tab.detail_view = ConfigDetailView::ItemFields;
                        app.config_tab.edit_field = 0;
                    }
                    _ => {}
                }
            }
        }

        // Vertical navigation
        KeyCode::Up | KeyCode::Char('k') => {
            if app.config_tab.focus == ConfigFocus::Sections {
                // Navigate sections
                let current = app.config_tab.section.index();
                if current > 0 {
                    app.config_tab.section = ConfigSection::from_index(current - 1);
                    app.config_tab.selected_item = 0;
                    app.config_tab.detail_view = ConfigDetailView::ItemList;
                }
            } else if in_field_view {
                // Navigate fields within an item
                if app.config_tab.edit_field > 0 {
                    app.config_tab.edit_field -= 1;
                }
            } else {
                // Navigate items in details
                if app.config_tab.selected_item > 0 {
                    app.config_tab.selected_item -= 1;
                    // Update scroll offset if needed
                    if app.config_tab.selected_item < app.config_tab.scroll_offset {
                        app.config_tab.scroll_offset = app.config_tab.selected_item;
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.config_tab.focus == ConfigFocus::Sections {
                // Navigate sections
                let current = app.config_tab.section.index();
                if current < ConfigSection::all().len() - 1 {
                    app.config_tab.section = ConfigSection::from_index(current + 1);
                    app.config_tab.selected_item = 0;
                    app.config_tab.detail_view = ConfigDetailView::ItemList;
                }
            } else if in_field_view {
                // Navigate fields within an item
                let max_fields = get_field_count(app);
                if app.config_tab.edit_field < max_fields.saturating_sub(1) {
                    app.config_tab.edit_field += 1;
                }
            } else {
                // Navigate items in details
                let max = get_config_item_count(app);
                if app.config_tab.selected_item < max.saturating_sub(1) {
                    app.config_tab.selected_item += 1;
                }
            }
        }

        // Toggle (for Disabled section)
        KeyCode::Char(' ') => {
            if app.config_tab.focus == ConfigFocus::Details
                && app.config_tab.section == ConfigSection::Disabled
            {
                if let Some(ref mut draft) = app.config_tab.config_draft {
                    draft.toggle_item(app.config_tab.selected_item);
                    app.config_tab.has_changes = true;
                }
            }
        }

        // Enter - edit or toggle depending on section and mode
        KeyCode::Enter => {
            if app.config_tab.focus == ConfigFocus::Sections {
                // Enter details panel
                app.config_tab.focus = ConfigFocus::Details;
                app.config_tab.selected_item = 0;
                app.config_tab.detail_view = ConfigDetailView::ItemList;
            } else if in_field_view {
                // Start editing the selected field
                start_field_edit(app);
            } else {
                // Section-specific enter behavior
                handle_config_enter(app);
            }
        }

        // Add new item
        KeyCode::Char('a') => {
            if app.config_tab.focus == ConfigFocus::Details && !in_field_view {
                handle_config_add(app);
            }
        }

        // Delete item
        KeyCode::Char('d') => {
            if app.config_tab.focus == ConfigFocus::Details && !in_field_view {
                // Guard: sections with fixed items can't delete
                let can_delete = match app.config_tab.section {
                    ConfigSection::Disabled | ConfigSection::Overrides | ConfigSection::Sail => {
                        false
                    }
                    ConfigSection::Logs => app.config_tab.selected_item >= 2,
                    _ => true,
                };
                if can_delete {
                    let count = get_config_item_count(app);
                    if count > 0 && app.config_tab.selected_item < count {
                        app.config_tab.confirm_delete = Some(app.config_tab.selected_item);
                    }
                }
            }
        }

        // Toggle enabled (for Custom processes)
        KeyCode::Char('e') => {
            if app.config_tab.focus == ConfigFocus::Details
                && app.config_tab.section == ConfigSection::Custom
            {
                if let Some(ref mut draft) = app.config_tab.config_draft {
                    if let Some(cp) = draft.custom.get_mut(app.config_tab.selected_item) {
                        cp.enabled = !cp.enabled;
                        app.config_tab.has_changes = true;
                    }
                }
            }
        }

        // Save configuration
        KeyCode::Char('s') => {
            if app.config_tab.has_changes && !in_field_view {
                save_config(app, working_dir);
            }
        }

        // Reset changes
        KeyCode::Char('r') => {
            if !in_field_view {
                app.config_tab.config_draft =
                    Some(app::ConfigDraft::from_config(app.config.as_ref()));
                app.config_tab.has_changes = false;
                app.config_tab.selected_item = 0;
                app.config_tab.scroll_offset = 0;
                app.config_tab.detail_view = ConfigDetailView::ItemList;
                app.set_status("Changes reset");
            }
        }

        // Escape - go back through view hierarchy
        KeyCode::Esc => {
            if in_field_view {
                // Exit field view back to item list
                app.config_tab.detail_view = ConfigDetailView::ItemList;
                app.config_tab.edit_field = 0;
            } else if app.config_tab.focus == ConfigFocus::Details {
                app.config_tab.focus = ConfigFocus::Sections;
            }
        }

        _ => {}
    }
}

/// Get the number of editable fields for the current section/item
fn get_field_count(app: &App) -> usize {
    use app::ConfigSection;

    match app.config_tab.section {
        ConfigSection::Overrides => 4,
        ConfigSection::Custom => 7,
        ConfigSection::Logs => 2,
        ConfigSection::QualityCustomTools => 5,
        ConfigSection::QualityDefaultArgs => 2,
        _ => 0,
    }
}

/// Start editing the currently selected field
fn start_field_edit(app: &mut App) {
    use app::{ConfigEditMode, ConfigSection};

    let field = app.config_tab.edit_field;
    let selected = app.config_tab.selected_item;

    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    // Check if this is an enum field
    let is_enum_field = match app.config_tab.section {
        ConfigSection::Overrides => field == 3,
        ConfigSection::Custom => field == 6,
        ConfigSection::QualityCustomTools => field == 4,
        _ => false,
    };

    if is_enum_field {
        match app.config_tab.section {
            ConfigSection::QualityCustomTools => {
                let current = draft
                    .quality
                    .custom_tools
                    .get(selected)
                    .map(|t| t.category.as_str())
                    .unwrap_or("quality");
                app.config_tab.enum_selection = if current == "testing" { 1 } else { 0 };
            }
            ConfigSection::Custom => {
                let current_policy = draft.custom.get(selected).map(|cp| cp.restart_policy);
                app.config_tab.enum_selection = match current_policy {
                    Some(config::RestartPolicy::Never) => 0,
                    Some(config::RestartPolicy::OnFailure) => 1,
                    Some(config::RestartPolicy::Always) => 2,
                    None => 0,
                };
            }
            _ => {
                app.config_tab.enum_selection = 0;
            }
        }
        app.config_tab.edit_mode = ConfigEditMode::SelectOption;
        return;
    }

    // Pre-fill the edit buffer with the current value for text fields
    let current_value = match app.config_tab.section {
        ConfigSection::Custom => {
            if let Some(cp) = draft.custom.get(selected) {
                match field {
                    0 => cp.name.clone(),
                    1 => cp.display_name.clone(),
                    2 => cp.command.clone(),
                    3 => cp.args.clone(),
                    4 => cp.hotkey.clone(),
                    5 => cp.working_dir.clone(),
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }
        ConfigSection::QualityCustomTools => {
            if let Some(tool) = draft.quality.custom_tools.get(selected) {
                match field {
                    0 => tool.name.clone(),
                    1 => tool.display_name.clone(),
                    2 => tool.command.clone(),
                    3 => tool.args.clone(),
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }
        ConfigSection::QualityDefaultArgs => {
            if let Some((tool_name, tool_args)) = draft.quality.default_args.get(selected) {
                match field {
                    0 => tool_name.clone(),
                    1 => tool_args.clone(),
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    app.config_tab.edit_buffer = current_value;
    app.config_tab.edit_mode = ConfigEditMode::EditText;
}

/// Get the number of items in the current config section
fn get_config_item_count(app: &App) -> usize {
    use app::ConfigSection;

    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return 0,
    };

    match app.config_tab.section {
        ConfigSection::Disabled => 5,
        ConfigSection::Overrides => 25, // 5 processes x (1 header + 4 fields)
        ConfigSection::Custom => draft.custom.len(),
        ConfigSection::Sail => 1,
        ConfigSection::Logs => 2 + draft.logs.files.len(),
        ConfigSection::QualityDisabledTools => draft.quality.disabled_tools.len(),
        ConfigSection::QualityCustomTools => draft.quality.custom_tools.len(),
        ConfigSection::QualityDefaultArgs => draft.quality.default_args.len(),
        ConfigSection::ArtisanFavorites => draft.artisan_favorites.len(),
        ConfigSection::MakeFavorites => draft.make_favorites.len(),
    }
}

/// Handle Enter key in config details (item list mode)
fn handle_config_enter(app: &mut App) {
    use app::{ConfigDetailView, ConfigEditMode, ConfigSection};

    match app.config_tab.section {
        ConfigSection::Disabled => {
            if let Some(ref mut draft) = app.config_tab.config_draft {
                draft.toggle_item(app.config_tab.selected_item);
                app.config_tab.has_changes = true;
            }
        }
        ConfigSection::Overrides => {
            // Flat navigation: compute process and field from selected_item
            let row = app.config_tab.selected_item % 5;
            if row == 0 {
                return; // Header row, do nothing
            }
            let process_idx = app.config_tab.selected_item / 5;
            let field_idx = row - 1; // 0=command, 1=args, 2=working_dir, 3=restart

            if field_idx == 3 {
                // Restart policy - enum select
                let draft = match &app.config_tab.config_draft {
                    Some(d) => d,
                    None => return,
                };
                let processes = ["serve", "vite", "queue", "horizon", "reverb"];
                let current_policy = processes
                    .get(process_idx)
                    .and_then(|name| draft.overrides.get(*name).map(|ovr| ovr.restart_policy));
                app.config_tab.enum_selection = match current_policy {
                    Some(config::RestartPolicy::Never) => 0,
                    Some(config::RestartPolicy::OnFailure) => 1,
                    Some(config::RestartPolicy::Always) => 2,
                    None => 0,
                };
                app.config_tab.edit_mode = ConfigEditMode::SelectOption;
            } else {
                // Text edit
                let draft = match &app.config_tab.config_draft {
                    Some(d) => d,
                    None => return,
                };
                let processes = ["serve", "vite", "queue", "horizon", "reverb"];
                let current_value = processes
                    .get(process_idx)
                    .and_then(|name| draft.overrides.get(*name))
                    .map(|ovr| match field_idx {
                        0 => ovr.command.clone(),
                        1 => ovr.args.clone(),
                        2 => ovr.working_dir.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                app.config_tab.edit_buffer = current_value;
                app.config_tab.edit_mode = ConfigEditMode::EditText;
            }
        }
        ConfigSection::Custom => {
            if app
                .config_tab
                .config_draft
                .as_ref()
                .map(|d| !d.custom.is_empty())
                .unwrap_or(false)
            {
                app.config_tab.detail_view = ConfigDetailView::ItemFields;
                app.config_tab.edit_field = 0;
            }
        }
        ConfigSection::Sail => {
            let draft = match &app.config_tab.config_draft {
                Some(d) => d,
                None => return,
            };
            app.config_tab.enum_selection = match draft.sail {
                None => 0,
                Some(true) => 1,
                Some(false) => 2,
            };
            app.config_tab.edit_mode = ConfigEditMode::SelectOption;
        }
        ConfigSection::Logs => {
            let item = app.config_tab.selected_item;
            if item == 0 {
                let current = app
                    .config_tab
                    .config_draft
                    .as_ref()
                    .map(|d| d.logs.max_lines.clone())
                    .unwrap_or_default();
                app.config_tab.edit_buffer = current;
                app.config_tab.edit_mode = ConfigEditMode::EditText;
            } else if item == 1 {
                let current = app
                    .config_tab
                    .config_draft
                    .as_ref()
                    .map(|d| d.logs.default_filter.clone())
                    .unwrap_or_default();
                let log_levels = [
                    "(none)",
                    "debug",
                    "info",
                    "notice",
                    "warning",
                    "error",
                    "critical",
                    "alert",
                    "emergency",
                ];
                app.config_tab.enum_selection = log_levels
                    .iter()
                    .position(|l| {
                        if current.is_empty() {
                            *l == "(none)"
                        } else {
                            *l == current.as_str()
                        }
                    })
                    .unwrap_or(0);
                app.config_tab.edit_mode = ConfigEditMode::SelectOption;
            } else {
                let file_idx = item - 2;
                let current = app
                    .config_tab
                    .config_draft
                    .as_ref()
                    .and_then(|d| d.logs.files.get(file_idx).cloned())
                    .unwrap_or_default();
                app.config_tab.edit_buffer = current;
                app.config_tab.edit_mode = ConfigEditMode::EditText;
            }
        }
        ConfigSection::QualityDisabledTools => {
            let current = app
                .config_tab
                .config_draft
                .as_ref()
                .and_then(|d| {
                    d.quality
                        .disabled_tools
                        .get(app.config_tab.selected_item)
                        .cloned()
                })
                .unwrap_or_default();
            app.config_tab.edit_buffer = current;
            app.config_tab.edit_mode = ConfigEditMode::EditText;
        }
        ConfigSection::QualityCustomTools => {
            if app
                .config_tab
                .config_draft
                .as_ref()
                .map(|d| !d.quality.custom_tools.is_empty())
                .unwrap_or(false)
            {
                app.config_tab.detail_view = ConfigDetailView::ItemFields;
                app.config_tab.edit_field = 0;
            }
        }
        ConfigSection::QualityDefaultArgs => {
            if app
                .config_tab
                .config_draft
                .as_ref()
                .map(|d| !d.quality.default_args.is_empty())
                .unwrap_or(false)
            {
                app.config_tab.detail_view = ConfigDetailView::ItemFields;
                app.config_tab.edit_field = 0;
            }
        }
        ConfigSection::ArtisanFavorites | ConfigSection::MakeFavorites => {
            let current = app
                .config_tab
                .config_draft
                .as_ref()
                .and_then(|d| {
                    let favs = if app.config_tab.section == ConfigSection::ArtisanFavorites {
                        &d.artisan_favorites
                    } else {
                        &d.make_favorites
                    };
                    favs.get(app.config_tab.selected_item).cloned()
                })
                .unwrap_or_default();
            app.config_tab.edit_buffer = current;
            app.config_tab.edit_mode = ConfigEditMode::EditText;
        }
    }
}

/// Handle adding new items in config
fn handle_config_add(app: &mut App) {
    use app::{ConfigSection, CustomProcessDraft, CustomToolDraft};

    let draft = match app.config_tab.config_draft.as_mut() {
        Some(d) => d,
        None => return,
    };

    match app.config_tab.section {
        ConfigSection::Custom => {
            draft.custom.push(CustomProcessDraft::new());
            app.config_tab.selected_item = draft.custom.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.detail_view = app::ConfigDetailView::ItemFields;
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
            app.config_tab.edit_field = 0;
            app.config_tab.edit_buffer.clear();
        }
        ConfigSection::Logs => {
            draft.logs.files.push(String::new());
            app.config_tab.selected_item = 2 + draft.logs.files.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.edit_buffer.clear();
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
        }
        ConfigSection::QualityDisabledTools => {
            draft.quality.disabled_tools.push(String::new());
            app.config_tab.selected_item = draft.quality.disabled_tools.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.edit_buffer.clear();
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
        }
        ConfigSection::QualityCustomTools => {
            draft
                .quality
                .custom_tools
                .push(CustomToolDraft::new_quality());
            app.config_tab.selected_item = draft.quality.custom_tools.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.detail_view = app::ConfigDetailView::ItemFields;
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
            app.config_tab.edit_field = 0;
            app.config_tab.edit_buffer.clear();
        }
        ConfigSection::QualityDefaultArgs => {
            draft
                .quality
                .default_args
                .push((String::new(), String::new()));
            app.config_tab.selected_item = draft.quality.default_args.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.detail_view = app::ConfigDetailView::ItemFields;
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
            app.config_tab.edit_field = 0;
            app.config_tab.edit_buffer.clear();
        }
        ConfigSection::ArtisanFavorites => {
            draft.artisan_favorites.push(String::new());
            app.config_tab.selected_item = draft.artisan_favorites.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.edit_buffer.clear();
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
        }
        ConfigSection::MakeFavorites => {
            draft.make_favorites.push(String::new());
            app.config_tab.selected_item = draft.make_favorites.len() - 1;
            app.config_tab.has_changes = true;
            app.config_tab.edit_buffer.clear();
            app.config_tab.edit_mode = app::ConfigEditMode::EditText;
        }
        ConfigSection::Disabled | ConfigSection::Overrides | ConfigSection::Sail => {}
    }
}

/// Apply enum selection to the config
fn apply_enum_selection(app: &mut App) {
    use app::ConfigSection;

    let selected = app.config_tab.selected_item;
    let enum_idx = app.config_tab.enum_selection;

    let draft = match app.config_tab.config_draft.as_mut() {
        Some(d) => d,
        None => return,
    };

    match app.config_tab.section {
        ConfigSection::Sail => {
            draft.sail = match enum_idx {
                0 => None,
                1 => Some(true),
                2 => Some(false),
                _ => None,
            };
        }
        ConfigSection::Logs => {
            let log_levels = [
                "(none)",
                "debug",
                "info",
                "notice",
                "warning",
                "error",
                "critical",
                "alert",
                "emergency",
            ];
            if let Some(level) = log_levels.get(enum_idx) {
                draft.logs.default_filter = if *level == "(none)" {
                    String::new()
                } else {
                    level.to_string()
                };
            }
        }
        ConfigSection::QualityCustomTools => {
            let categories = ["quality", "testing"];
            if let Some(cat) = categories.get(enum_idx) {
                if let Some(tool) = draft.quality.custom_tools.get_mut(selected) {
                    tool.category = cat.to_string();
                }
            }
        }
        ConfigSection::Overrides => {
            // Flat layout: compute process index from selected_item
            let process_idx = selected / 5;
            let new_policy = match enum_idx {
                0 => config::RestartPolicy::Never,
                1 => config::RestartPolicy::OnFailure,
                2 => config::RestartPolicy::Always,
                _ => config::RestartPolicy::Never,
            };
            let processes = ["serve", "vite", "queue", "horizon", "reverb"];
            if let Some(name) = processes.get(process_idx) {
                let ovr = draft.get_or_create_override(name);
                ovr.restart_policy = new_policy;
            }
        }
        ConfigSection::Custom => {
            let new_policy = match enum_idx {
                0 => config::RestartPolicy::Never,
                1 => config::RestartPolicy::OnFailure,
                2 => config::RestartPolicy::Always,
                _ => config::RestartPolicy::Never,
            };
            if let Some(cp) = draft.custom.get_mut(selected) {
                cp.restart_policy = new_policy;
            }
        }
        _ => {}
    }
}

/// Apply the current edit buffer to the config
fn apply_config_edit(app: &mut App) {
    use app::ConfigSection;

    let value = app.config_tab.edit_buffer.clone();
    let field = app.config_tab.edit_field;
    let selected = app.config_tab.selected_item;

    let draft = match app.config_tab.config_draft.as_mut() {
        Some(d) => d,
        None => return,
    };

    match app.config_tab.section {
        ConfigSection::Overrides => {
            // Flat layout: compute process and field from selected_item
            let row = selected % 5;
            if row == 0 {
                return;
            }
            let process_idx = selected / 5;
            let field_idx = row - 1;
            let processes = ["serve", "vite", "queue", "horizon", "reverb"];
            if let Some(name) = processes.get(process_idx) {
                let ovr = draft.get_or_create_override(name);
                match field_idx {
                    0 => ovr.command = value,
                    1 => ovr.args = value,
                    2 => ovr.working_dir = value,
                    _ => {}
                }
            }
        }
        ConfigSection::Custom => {
            if let Some(cp) = draft.custom.get_mut(selected) {
                match field {
                    0 => cp.name = value,
                    1 => cp.display_name = value,
                    2 => cp.command = value,
                    3 => cp.args = value,
                    4 => {
                        cp.hotkey = value
                            .chars()
                            .next()
                            .map(|c| c.to_string())
                            .unwrap_or_default()
                    }
                    5 => cp.working_dir = value,
                    _ => {}
                }
            }
        }
        ConfigSection::Logs => {
            if selected == 0 {
                draft.logs.max_lines = value;
            } else if selected >= 2 {
                let file_idx = selected - 2;
                if let Some(file) = draft.logs.files.get_mut(file_idx) {
                    *file = value;
                }
            }
        }
        ConfigSection::QualityDisabledTools => {
            if let Some(tool) = draft.quality.disabled_tools.get_mut(selected) {
                *tool = value;
            }
        }
        ConfigSection::QualityCustomTools => {
            if let Some(tool) = draft.quality.custom_tools.get_mut(selected) {
                match field {
                    0 => tool.name = value,
                    1 => tool.display_name = value,
                    2 => tool.command = value,
                    3 => tool.args = value,
                    _ => {}
                }
            }
        }
        ConfigSection::QualityDefaultArgs => {
            if let Some((tool_name, tool_args)) = draft.quality.default_args.get_mut(selected) {
                match field {
                    0 => *tool_name = value,
                    1 => *tool_args = value,
                    _ => {}
                }
            }
        }
        ConfigSection::ArtisanFavorites => {
            if let Some(fav) = draft.artisan_favorites.get_mut(selected) {
                *fav = value;
            }
        }
        ConfigSection::MakeFavorites => {
            if let Some(fav) = draft.make_favorites.get_mut(selected) {
                *fav = value;
            }
        }
        ConfigSection::Disabled | ConfigSection::Sail => {}
    }
}

/// Perform delete operation on config
fn perform_config_delete(app: &mut App, idx: usize) {
    use app::ConfigSection;

    let draft = match app.config_tab.config_draft.as_mut() {
        Some(d) => d,
        None => return,
    };

    match app.config_tab.section {
        ConfigSection::Custom => {
            if idx < draft.custom.len() {
                draft.custom.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.custom.len()
                {
                    app.config_tab.selected_item = draft.custom.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::Logs => {
            if idx >= 2 {
                let file_idx = idx - 2;
                if file_idx < draft.logs.files.len() {
                    draft.logs.files.remove(file_idx);
                    if app.config_tab.selected_item > 0
                        && app.config_tab.selected_item >= 2 + draft.logs.files.len()
                    {
                        app.config_tab.selected_item =
                            (2 + draft.logs.files.len()).saturating_sub(1);
                    }
                }
            }
        }
        ConfigSection::QualityDisabledTools => {
            if idx < draft.quality.disabled_tools.len() {
                draft.quality.disabled_tools.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.quality.disabled_tools.len()
                {
                    app.config_tab.selected_item =
                        draft.quality.disabled_tools.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::QualityCustomTools => {
            if idx < draft.quality.custom_tools.len() {
                draft.quality.custom_tools.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.quality.custom_tools.len()
                {
                    app.config_tab.selected_item =
                        draft.quality.custom_tools.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::QualityDefaultArgs => {
            if idx < draft.quality.default_args.len() {
                draft.quality.default_args.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.quality.default_args.len()
                {
                    app.config_tab.selected_item =
                        draft.quality.default_args.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::ArtisanFavorites => {
            if idx < draft.artisan_favorites.len() {
                draft.artisan_favorites.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.artisan_favorites.len()
                {
                    app.config_tab.selected_item = draft.artisan_favorites.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::MakeFavorites => {
            if idx < draft.make_favorites.len() {
                draft.make_favorites.remove(idx);
                if app.config_tab.selected_item > 0
                    && app.config_tab.selected_item >= draft.make_favorites.len()
                {
                    app.config_tab.selected_item = draft.make_favorites.len().saturating_sub(1);
                }
            }
        }
        ConfigSection::Disabled | ConfigSection::Overrides | ConfigSection::Sail => {}
    }
}

/// Save configuration to file
fn save_config(app: &mut App, working_dir: &Path) {
    let draft = match &app.config_tab.config_draft {
        Some(d) => d,
        None => return,
    };

    // Convert draft to config
    let new_config = draft.to_config();

    // Validate before saving
    // We'll do a basic validation here - the full validation happens on load
    let config_path = working_dir.join(".laramux.json");
    match serde_json::to_string_pretty(&new_config) {
        Ok(content) => match std::fs::write(&config_path, content) {
            Ok(_) => {
                app.config_tab.has_changes = false;
                app.config_tab.error = None;
                // Update the live config
                app.config = Some(new_config);
                app.set_status("Configuration saved to .laramux.json");
            }
            Err(e) => {
                app.config_tab.error = Some(format!("Failed to write: {}", e));
            }
        },
        Err(e) => {
            app.config_tab.error = Some(format!("Failed to serialize: {}", e));
        }
    }
}

/// Format a config loading error into a user-friendly message
fn format_config_error(error: &error::LaraMuxError) -> String {
    use error::LaraMuxError;

    match error {
        LaraMuxError::JsonParse(e) => {
            // Parse serde_json error for line/column info
            let line = e.line();
            let column = e.column();
            let classify = e.classify();

            let error_type = match classify {
                serde_json::error::Category::Io => "IO error",
                serde_json::error::Category::Syntax => "Syntax error",
                serde_json::error::Category::Data => "Invalid data",
                serde_json::error::Category::Eof => "Unexpected end of file",
            };

            format!(
                "{} in .laramux.json at line {}, column {}: {}",
                error_type, line, column, e
            )
        }
        LaraMuxError::ConfigValidation(msg) => {
            format!("Configuration error: {}", msg)
        }
        LaraMuxError::Io(e) => {
            format!("Failed to read .laramux.json: {}", e)
        }
        _ => format!("Configuration error: {}", error),
    }
}
