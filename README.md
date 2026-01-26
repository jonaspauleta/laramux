# LaraMux

A terminal UI application for managing Laravel development processes in a single terminal window.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Unified Process Management** - Run `artisan serve`, Vite, queue workers, and Reverb in one terminal
- **Auto-Discovery** - Automatically detects available services from `composer.json` and `package.json`
- **Real-time Log Viewing** - Watch `storage/logs/laravel.log` updates in real-time
- **Smart Package Manager Detection** - Detects npm, yarn, pnpm, or bun for running Vite
- **Hotkey Controls** - Quickly restart individual processes or all at once
- **Graceful Shutdown** - Properly terminates all child processes on exit

## Screenshot

```
┌─ Processes ─────┐┌─ Serve Output 🟢 ─────────────────────────────────┐
│▶ 🟢 Serve   [s] ││ Started Serve (PID: 12345)                       │
│  🟢 Vite    [v] ││ INFO  Server running on [http://127.0.0.1:8000]  │
│  🟢 Queue   [q] ││                                                   │
│  ⚫ Reverb  [b] ││                                                   │
│                 ││                                                   │
└─────────────────┘└───────────────────────────────────────────────────┘
                   ┌─ Laravel Log (42 lines) ──────────────────────────┐
                   │ [2024-01-15 10:30:45] local.INFO: User logged in  │
                   │ [2024-01-15 10:30:46] local.DEBUG: Query executed │
                   └───────────────────────────────────────────────────┘
↑↓:Navigate  q:Queue  v:Vite  s:Serve  b:Reverb  r:Restart All  c:Clear  Ctrl+C:Quit
```

## Installation

Requires Rust 1.70 or later.

### Option 1: Run with `--manifest-path`

```bash
cd /path/to/laravel-project
cargo run --manifest-path /path/to/laramux/Cargo.toml
```

### Option 2: Build and install the binary (Recommended)

```bash
cd /path/to/laramux
cargo install --path .

# Then run from any Laravel project:
cd /path/to/laravel-project
laramux
```

### Option 3: Build release and copy binary

```bash
cd /path/to/laramux
cargo build --release
# Binary is at: target/release/laramux

# Copy to a directory in your PATH:
cp target/release/laramux /usr/local/bin/

# Then run from any Laravel project:
cd /path/to/laravel-project
laramux
```

### Option 4: Create an alias

```bash
# Add to ~/.zshrc or ~/.bashrc:
alias laramux='cargo run --manifest-path /path/to/laramux/Cargo.toml'

# Then:
cd /path/to/laravel-project
laramux
```

> **Note:** Option 2 (`cargo install --path .`) is the cleanest approach for regular use.

## Usage

Navigate to your Laravel project directory and run:

```bash
cd /path/to/your/laravel-project
laramux
```

### Requirements

LaraMux expects to be run from a Laravel project root containing:
- `composer.json` with `laravel/framework` as a dependency
- Optionally `package.json` with Vite for frontend assets

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate process list |
| `s` | Restart Laravel serve |
| `v` | Restart Vite dev server |
| `q` | Restart queue worker |
| `b` | Restart Reverb (websockets) |
| `r` | Restart all processes |
| `c` | Clear selected process output |
| `Page Up` / `Page Down` | Scroll output |
| `Ctrl+C` | Quit and stop all processes |

## Detected Services

LaraMux automatically detects and manages:

| Service | Detection | Command |
|---------|-----------|---------|
| **Serve** | Always (Laravel project) | `php artisan serve` |
| **Queue** | Always (Laravel project) | `php artisan queue:work --tries=3` |
| **Vite** | `vite` in package.json | `npm/yarn/pnpm/bun run dev` |
| **Reverb** | `laravel/reverb` in composer.json | `php artisan reverb:start` |

## Configuration

Currently, LaraMux uses sensible defaults. Custom configuration support is planned for future releases.

## Troubleshooting

### "composer.json not found"
Make sure you're running LaraMux from your Laravel project's root directory.

### "Not a Laravel project"
Ensure `laravel/framework` is listed in your `composer.json` dependencies.

### Vite not starting
Check that:
- `package.json` exists with a `dev` script
- `vite` or `laravel-vite-plugin` is in your devDependencies
- Node modules are installed (`npm install`)

### Processes not stopping on exit
LaraMux sends SIGTERM and waits 5 seconds before SIGKILL. If processes persist, they may be ignoring signals.

## Development

```bash
# Run in development
cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
