# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo build              # Build debug
cargo build --release    # Build release
cargo run               # Run in development
cargo test              # Run tests
cargo test <name>       # Run single test
cargo fmt --check       # Check formatting
cargo clippy --all-features -- -D warnings  # Lint (CI enforces -D warnings)
```

## Architecture Overview

LaraMux is a TUI application (~1,700 lines) for managing Laravel development processes in a single terminal. It uses an async event-driven architecture with tokio.

### Module Structure

```
src/
├── main.rs      # Event loop: spawns 4 async tasks (input, tick, log watcher, process manager)
├── app.rs       # App state: processes, logs, UI selection, status messages
├── event.rs     # Event enum (Input, Tick, ProcessOutput, LogUpdate, Resize)
├── error.rs     # LaraMuxError enum with thiserror
├── tui.rs       # Terminal init/restore via crossterm
├── process/
│   ├── types.rs     # ProcessKind enum, Process struct, ProcessConfig, ProcessStatus
│   ├── discovery.rs # Auto-detects Laravel services from composer.json/package.json
│   └── manager.rs   # Spawns/kills processes, pipes stdout/stderr to event channel
├── log/
│   ├── watcher.rs   # notify-based file watcher for storage/logs/laravel.log
│   └── parser.rs    # Parses Laravel log format, extracts level/timestamp/message
└── ui/
    ├── layout.rs    # 4-area layout: sidebar (20%) + output/logs (80%) + statusbar
    ├── sidebar.rs   # Process list with status indicators
    ├── output.rs    # Selected process stdout/stderr display
    ├── logstream.rs # Laravel log display
    └── statusbar.rs # Transient status messages
```

### Key Data Flow

1. `main.rs` spawns async tasks that send events via `mpsc` channel
2. Event loop receives events, updates `App` state, renders UI with ratatui
3. Process output streams are read by dedicated tasks, forwarded as `ProcessOutput` events
4. Log watcher uses `notify` crate, reads file incrementally, sends `LogUpdate` events

### Process Management

- Processes: Serve, Vite, Queue, Horizon, Reverb
- Discovery in `discovery.rs` checks `composer.json` for Laravel packages
- Graceful shutdown: SIGTERM → 5s wait → SIGKILL (Unix via nix crate)
- macOS: Skips `artisan serve` if Laravel Herd detected

### Key Dependencies

- **ratatui/crossterm**: TUI rendering and terminal handling
- **tokio**: Async runtime for concurrent process management
- **notify**: Filesystem watching for Laravel logs
- **nix** (Unix): Signal handling for process termination
