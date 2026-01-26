# LaraMux Implementation Plan

## Overview
Build a Rust TUI application that manages Laravel development processes (serve, vite, queue, reverb) in a single terminal with real-time log viewing.

## Module Structure

```
laramux/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, event loop
│   ├── app.rs               # Application state
│   ├── tui.rs               # Terminal setup/teardown
│   ├── event.rs             # Event types
│   ├── error.rs             # Custom errors
│   ├── process/
│   │   ├── mod.rs
│   │   ├── manager.rs       # Spawn/restart/kill processes
│   │   ├── discovery.rs     # Auto-detect services
│   │   └── types.rs         # Process, ProcessStatus, ProcessConfig
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs        # 3-pane layout
│   │   ├── sidebar.rs       # Process list widget
│   │   ├── output.rs        # Process output view
│   │   └── logstream.rs     # Laravel log pane
│   └── log/
│       ├── mod.rs
│       ├── watcher.rs       # File watching for laravel.log
│       └── parser.rs        # Stack trace detection
```

## Core Architecture

### Event-Driven Design
- Single `mpsc` channel for all events (input, process output, log updates)
- Main loop uses `tokio::select!` over event channel + tick interval
- `CancellationToken` for graceful shutdown of all async tasks

### Key Data Structures

**ProcessKind**: `Serve | Vite | Queue | Reverb`

**ProcessStatus**: `Running | Stopped | Restarting | Failed`

**Event**:
- `Input(KeyEvent)` - keyboard input
- `ProcessOutput { kind, line, is_stderr }` - process stdout/stderr
- `ProcessExited { kind, exit_code }` - process termination
- `LogUpdate(Vec<String>)` - new laravel.log content

**App State**:
- `processes: HashMap<ProcessKind, Process>` - managed processes
- `selected_index: usize` - sidebar selection
- `log_lines: VecDeque<LogLine>` - last 10 log lines
- `should_quit: bool` - exit flag

## Implementation Phases

### Phase 1: Foundation
1. Create Cargo.toml with dependencies
2. Create `error.rs` with `LaraMuxError` enum
3. Create `process/types.rs` - Process, ProcessKind, ProcessStatus, ProcessConfig
4. Create `event.rs` - Event enum
5. Create `app.rs` - App state struct with methods

### Phase 2: TUI Shell
1. Create `tui.rs` - terminal setup/teardown (crossterm)
2. Create `ui/layout.rs` - 3-pane layout (20% sidebar | 70% output / 30% log)
3. Create `ui/sidebar.rs` - process list with status indicators (🟢/🔴)
4. Create `ui/output.rs` - scrollable output view
5. Create `ui/logstream.rs` - log pane (always visible)
6. Create `ui/mod.rs` - main render function

### Phase 3: Process Management
1. Create `process/discovery.rs` - parse composer.json/package.json
2. Create `process/manager.rs` - ProcessManager with spawn/kill/restart
3. Implement output reader tasks (spawn tokio tasks for stdout/stderr)
4. Implement graceful kill (SIGTERM → wait 5s → SIGKILL)

### Phase 4: Log Watching
1. Create `log/parser.rs` - detect stack traces, error levels
2. Create `log/watcher.rs` - notify-based file watching, seek-based reading

### Phase 5: Integration
1. Wire up main.rs event loop with `tokio::select!`
2. Implement input handler task (crossterm EventStream)
3. Implement hotkeys: `q` (restart queue), `v` (restart vite), `r` (restart all), `c` (clear)
4. Implement Up/Down arrow navigation
5. Implement Ctrl+C graceful shutdown

### Phase 6: Polish
1. Add error highlighting (red) in output for stack traces
2. Add status bar with hotkey hints
3. Handle edge cases (missing composer.json, permission errors)
4. Test with real Laravel project

## Dependencies (Cargo.toml)

```toml
[package]
name = "laramux"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = { version = "0.28", features = ["event-stream"] }
tokio = { version = "1.40", features = ["full"] }
tokio-util = "0.7"
notify = "6.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
anyhow = "1.0"
futures = "0.3"
```

## Critical Files

| File | Purpose |
|------|---------|
| `src/app.rs` | Central state; all modules depend on App struct |
| `src/process/manager.rs` | Core process lifecycle, most complex async logic |
| `src/event.rs` | Event enum - communication contract between tasks |
| `src/main.rs` | Event loop - ties all components together |
| `src/ui/layout.rs` | 3-pane structure defining visual organization |

## Verification

1. **Startup test**: Run `cargo run` in a Laravel project root
   - Should detect and start serve, vite, queue processes
   - TUI should render 3-pane layout without flickering

2. **Navigation test**: Press Up/Down arrows
   - Sidebar selection should change
   - Main view should show selected process output

3. **Log test**: In Laravel, run `Log::info('test')`
   - Bottom pane should update within 1 second

4. **Hotkey test**: Press `q`
   - Queue worker should restart (status indicator changes)

5. **Shutdown test**: Press Ctrl+C
   - All child processes should terminate
   - Terminal should restore cleanly

## Key Design Decisions

- **Single event channel**: Simplifies event loop, avoids complex sync
- **`kill_on_drop(true)`**: Auto-cleanup on panic
- **VecDeque ring buffer**: Bounded output memory (1000 lines max)
- **Stateless UI**: Render functions take `&App` - no widget state
- **Seek-based log reading**: Only reads new content, not full file
