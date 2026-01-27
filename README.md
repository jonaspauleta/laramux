# LaraMux

A terminal UI application for managing Laravel development processes in a single terminal window.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)
![Downloads](https://img.shields.io/github/downloads/jonaspauleta/laramux/v1.0.0/total)

![LaraMux Preview](public/preview.gif)

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Homebrew](#homebrew-macos--linux)
  - [apt (Debian/Ubuntu)](#apt-debianubuntu)
  - [dnf (Fedora/RHEL)](#dnf-fedorarhel)
  - [Download Binary](#download-binary)
  - [Build from Source](#build-from-source)
- [Usage](#usage)
  - [Requirements](#requirements)
  - [Keyboard Controls](#keyboard-controls)
- [Detected Services](#detected-services)
- [Configuration](#configuration)
  - [Disable Built-in Processes](#disable-built-in-processes)
  - [Override Process Commands](#override-process-commands)
  - [Add Custom Processes](#add-custom-processes)
  - [Quality Tools Configuration](#quality-tools-configuration)
  - [Log Configuration](#log-configuration)
  - [Artisan Configuration](#artisan-configuration)
  - [Restart Policies](#restart-policies)
  - [Complete Example](#complete-example)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [License](#license)
- [Contributing](#contributing)

## Features

- **Unified Process Management** - Run `artisan serve`, Vite, queue workers, and Reverb in one terminal
- **Auto-Discovery** - Automatically detects available services from `composer.json` and `package.json`
- **Real-time Log Viewing** - Watch `storage/logs/laravel.log` updates in real-time
- **Smart Package Manager Detection** - Detects npm, yarn, pnpm, or bun for running Vite
- **Hotkey Controls** - Quickly restart individual processes or all at once
- **Custom Processes** - Add your own processes via [configuration](#configuration)
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
| `h` | Restart Horizon |
| `b` | Restart Reverb (websockets) |
| `r` | Restart all processes |
| `c` | Clear selected process output |
| `f` | Toggle favorite (Artisan/Make tabs) |
| `Page Up` / `Page Down` | Scroll output |
| `Ctrl+C` | Quit and stop all processes |

Custom processes can define their own hotkeys (see [Configuration](#configuration)).

## Detected Services

LaraMux automatically detects and manages:

| Service | Detection | Command |
|---------|-----------|---------|
| **Serve** | Always (unless Laravel Herd detected) | `php artisan serve` |
| **Queue** | Always (unless Horizon detected) | `php artisan queue:work --tries=3` |
| **Horizon** | `laravel/horizon` in composer.json | `php artisan horizon` |
| **Vite** | `vite` in package.json | `npm/yarn/pnpm/bun run dev` |
| **Reverb** | `laravel/reverb` in composer.json | `php artisan reverb:start` |

> **Note:** If [Laravel Herd](https://herd.laravel.com) is installed on macOS, LaraMux will skip `artisan serve` since Herd handles serving automatically.

## Configuration

Create a `.laramux.json` file in your Laravel project root to customize LaraMux behavior. All sections are optional.

For IDE autocompletion and validation, add the schema reference:

```json
{
  "$schema": "https://raw.githubusercontent.com/jonaspauleta/laramux/main/.laramux.schema.json"
}
```

### Disable Built-in Processes

```json
{
  "disabled": {
    "serve": true,
    "queue": true
  }
}
```

Available processes to disable: `serve`, `vite`, `queue`, `horizon`, `reverb`

### Override Process Commands

Override command, arguments, working directory, environment variables, and restart behavior:

```json
{
  "overrides": {
    "serve": {
      "command": "php",
      "args": ["artisan", "serve", "--port=8080", "--host=0.0.0.0"],
      "working_dir": "backend",
      "env": {
        "APP_DEBUG": "true"
      },
      "restart_policy": "on_failure"
    },
    "queue": {
      "args": ["artisan", "queue:work", "--tries=5", "--timeout=90"],
      "restart_policy": "always"
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `command` | Override the executable |
| `args` | Override command arguments |
| `working_dir` | Relative path from project root (no `..` allowed) |
| `env` | Environment variables (keys must be alphanumeric with underscores) |
| `restart_policy` | `never` (default), `on_failure`, or `always` |

### Add Custom Processes

```json
{
  "custom": [
    {
      "name": "scheduler",
      "display_name": "Scheduler",
      "hotkey": "d",
      "command": "php",
      "args": ["artisan", "schedule:work"],
      "working_dir": "backend",
      "env": {
        "LOG_LEVEL": "debug"
      },
      "restart_policy": "always"
    },
    {
      "name": "octane",
      "display_name": "Octane",
      "hotkey": "o",
      "command": "php",
      "args": ["artisan", "octane:start", "--watch"]
    }
  ]
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier for the process |
| `display_name` | Yes | Name shown in the sidebar |
| `command` | Yes | Executable to run |
| `args` | No | Command arguments (default: `[]`) |
| `hotkey` | No | Single lowercase letter for quick restart |
| `enabled` | No | Set to `false` to disable (default: `true`) |
| `working_dir` | No | Relative path from project root |
| `env` | No | Environment variables |
| `restart_policy` | No | `never`, `on_failure`, or `always` |

**Reserved hotkeys:** `r` (restart all), `c` (clear output), `s`, `v`, `q`, `h`, `b` (built-in processes)

### Quality Tools Configuration

Customize the Quality tab tools - disable tools, add custom ones, or set default arguments:

```json
{
  "quality": {
    "disabled_tools": ["phpcs", "PHP_CodeSniffer"],
    "custom_tools": [
      {
        "name": "custom-lint",
        "display_name": "Custom Linter",
        "command": "./scripts/lint.sh",
        "args": ["--fix"],
        "category": "quality"
      },
      {
        "name": "dusk",
        "display_name": "Laravel Dusk",
        "command": "php",
        "args": ["artisan", "dusk"],
        "category": "testing"
      }
    ],
    "default_args": {
      "phpstan": ["--memory-limit=512M"],
      "pest": ["--parallel"]
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `disabled_tools` | List of tool display names to hide |
| `custom_tools` | Add custom quality or testing tools |
| `default_args` | Extra arguments to append (keyed by lowercase tool name) |

Custom tool fields:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier |
| `display_name` | Yes | Name shown in Quality tab |
| `command` | Yes | Executable to run |
| `args` | No | Command arguments |
| `category` | Yes | `quality` or `testing` |

### Log Configuration

Customize log viewing behavior:

```json
{
  "logs": {
    "max_lines": 500,
    "files": [
      "storage/logs/queue.log",
      "storage/logs/horizon.log"
    ],
    "default_filter": "warning"
  }
}
```

| Field | Description |
|-------|-------------|
| `max_lines` | Max log lines to keep (10-10000, default: 100) |
| `files` | Additional log files to watch (relative paths) |
| `default_filter` | Default level filter: `debug`, `info`, `notice`, `warning`, `error`, `critical`, `alert`, `emergency` |

### Artisan & Make Favorites

Mark your frequently used commands as favorites by pressing `f` while viewing them in the Artisan or Make tabs. Favorites appear at the top of the command list with a ★ indicator and are automatically saved to your config file.

You can also configure favorites manually:

```json
{
  "artisan": {
    "favorites": ["migrate:fresh", "cache:clear", "optimize:clear"]
  },
  "make": {
    "favorites": ["make:model", "make:controller", "make:migration"]
  }
}
```

| Section | Description |
|---------|-------------|
| `artisan.favorites` | Array of artisan command names to mark as favorites |
| `make.favorites` | Array of make command names to mark as favorites |

### Restart Policies

Processes can be configured to automatically restart:

| Policy | Behavior |
|--------|----------|
| `never` | Never auto-restart (default) |
| `on_failure` | Restart only if exit code is non-zero |
| `always` | Always restart regardless of exit code |

Auto-restart uses exponential backoff (2^failures seconds, max 60s) to prevent rapid restart loops.

### Complete Example

```json
{
  "$schema": "https://raw.githubusercontent.com/jonaspauleta/laramux/main/.laramux.schema.json",
  "disabled": {
    "serve": true
  },
  "overrides": {
    "queue": {
      "args": ["artisan", "queue:work", "--queue=high,default"],
      "restart_policy": "on_failure"
    }
  },
  "custom": [
    {
      "name": "scheduler",
      "display_name": "Scheduler",
      "hotkey": "d",
      "command": "php",
      "args": ["artisan", "schedule:work"],
      "restart_policy": "always"
    }
  ],
  "quality": {
    "disabled_tools": ["phpcs"],
    "default_args": {
      "phpstan": ["--memory-limit=512M"]
    }
  },
  "logs": {
    "max_lines": 500,
    "default_filter": "warning"
  },
  "artisan": {
    "favorites": ["migrate:fresh", "cache:clear"]
  },
  "make": {
    "favorites": ["make:model", "make:controller"]
  }
}
```

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
