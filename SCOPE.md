# Build MVP for "LaraMux" (Rust-based TUI for Laravel Process & Log Management)

**Status:** Triage / Todo
**Priority:** High
**Label:** Feature, Tooling, Rust

## Context & Problem
Developing modern Laravel applications (like Apex Scout) currently requires managing multiple fragmented terminal tabs. A typical session involves:
1. `php artisan serve` (HTTP)
2. `npm run dev` (Vite)
3. `php artisan queue:work` (Background jobs)
4. `php artisan reverb:start` (WebSockets)
5. `tail -f storage/logs/laravel.log` (Error logging)

This "tab fatigue" leads to missed errors (buried in logs) and friction when restarting individual services (finding the right tab to `Ctrl+C`).

## Proposed Solution
Build **LaraMux** (Laravel Multiplexer): A single-binary Rust CLI tool that runs all Laravel processes in one terminal window using a TUI (Terminal User Interface). It will multiplex process outputs and provide a dedicated, interactive pane for parsing `laravel.log` in real-time.

## MVP Scope (Phase 1)

### 1. The Process Manager (Backend)
* **Auto-Discovery:** On startup, parse `composer.json` and `package.json` to detect necessary services:
    * `vite` (if `package.json` has `dev` script).
    * `octane` or `serve` (if `laravel/octane` is present or default to artisan serve).
    * `reverb` (if `config/reverb.php` exists).
    * `queue` (default to `queue:work`).
* **Concurrency:** Use `tokio` to spawn these commands as async child processes.
* **Output Capture:** Pipe `stdout` and `stderr` from all children into a central buffer.

### 2. The TUI Layout (Frontend)
Implement a 3-pane layout using `ratatui`:
* **Sidebar (Left):** List of active processes with status indicators (🟢 Running, 🔴 Stopped).
* **Main View (Top-Right):** The raw output stream of the *currently selected* process.
* **Log Stream (Bottom-Right):** A dedicated, sticky pane that *always* shows the last 10 lines of `storage/logs/laravel.log`, regardless of which process is selected.

### 3. Interactive Features
* **Global Hotkeys:**
    * `q` → Restart Queue Worker (instant).
    * `v` → Restart Vite.
    * `r` → Restart All.
    * `c` → Clear Logs.
* **Error Parsing:** If `laravel.log` detects a stack trace, highlight the file path/line number in **Red**.

## Technical Implementation

* **Stack:** Rust (Latest Stable)
* **Key Crates:**
    * `ratatui`: For rendering the TUI interface.
    * `tokio`: For async runtime and `process::Command` management.
    * `notify`: For watching filesystem changes on `laravel.log`.
    * `crossterm`: For handling raw terminal input/events.
    * `serde_json`: For parsing `composer.json` / `package.json`.

## Acceptance Criteria
- [ ] Running `laramux` in the root of a Laravel project starts at least 3 processes (`serve`, `vite`, `queue`) automatically.
- [ ] The TUI renders correctly without flickering.
- [ ] User can switch between process outputs using `Up`/`Down` arrows.
- [ ] Modifying `storage/logs/laravel.log` (e.g., via `Log::info('test')`) instantly appears in the bottom pane.
- [ ] Pressing `q` kills and restarts the `php artisan queue:work` process immediately.
- [ ] `Ctrl+C` gracefully shuts down ALL child processes before exiting.