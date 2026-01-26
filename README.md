# LaraMux

A terminal UI application for managing Laravel development processes in a single terminal window.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

![LaraMux Preview](public/preview.gif)

## Features

- **Unified Process Management** - Run `artisan serve`, Vite, queue workers, and Reverb in one terminal
- **Auto-Discovery** - Automatically detects available services from `composer.json` and `package.json`
- **Real-time Log Viewing** - Watch `storage/logs/laravel.log` updates in real-time
- **Smart Package Manager Detection** - Detects npm, yarn, pnpm, or bun for running Vite
- **Hotkey Controls** - Quickly restart individual processes or all at once
- **Graceful Shutdown** - Properly terminates all child processes on exit

## Installation

### Homebrew (macOS & Linux)

```bash
brew tap jonaspauleta/tap
brew install laramux
```

### apt (Debian/Ubuntu)

```bash
# Download the .deb package
curl -LO https://github.com/jonaspauleta/laramux/releases/latest/download/laramux_amd64.deb

# Install
sudo dpkg -i laramux_amd64.deb
```

For ARM64 systems, use `laramux_arm64.deb` instead.

### dnf (Fedora/RHEL)

```bash
# Download the .rpm package
curl -LO https://github.com/jonaspauleta/laramux/releases/latest/download/laramux_x86_64.rpm

# Install
sudo dnf install ./laramux_x86_64.rpm
```

For ARM64 systems, use `laramux_aarch64.rpm` instead.

### Download Binary

Download pre-built binaries from [GitHub Releases](https://github.com/jonaspauleta/laramux/releases):

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `laramux-macos-aarch64` |
| macOS (Intel) | `laramux-macos-x86_64` |
| Linux (x86_64) | `laramux-linux-x86_64` |
| Linux (ARM64) | `laramux-linux-aarch64` |
| Windows | `laramux-windows-x86_64.exe` |

```bash
# Example: macOS Apple Silicon
curl -L https://github.com/jonaspauleta/laramux/releases/latest/download/laramux-macos-aarch64 -o laramux
chmod +x laramux
sudo mv laramux /usr/local/bin/
```

### Build from Source

Requires [Rust](https://rustup.rs/) 1.70 or later.

```bash
git clone https://github.com/jonaspauleta/laramux.git
cd laramux
cargo install --path .
```

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
| **Serve** | Always (unless Laravel Herd detected) | `php artisan serve` |
| **Queue** | Always (Laravel project) | `php artisan queue:work --tries=3` |
| **Vite** | `vite` in package.json | `npm/yarn/pnpm/bun run dev` |
| **Reverb** | `laravel/reverb` in composer.json | `php artisan reverb:start` |

> **Note:** If [Laravel Herd](https://herd.laravel.com) is installed on macOS, LaraMux will skip `artisan serve` since Herd handles serving automatically.

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
