# twm - Auto-Tiling Window Manager for Windows

A lightweight auto-tiling window manager for Windows 10/11. No keyboard shortcuts to learn — just run it and your windows automatically tile.

## How It Works

1. Run `twm.exe`
2. Open windows normally (browser, editor, terminal, etc.)
3. Windows are **automatically arranged in a tiling layout** — no overlap, no gaps
4. Close a window and the remaining windows **automatically re-tile** to fill the space

That's it. No shortcuts to memorize, no configuration required.

```
┌─────────────┬─────────────┐     ┌───────────┬───────────┐
│             │             │     │           │           │
│   Browser   │   Editor    │ ──→ │  Browser  │  Editor   │
│             │             │     │           ├───────────┤
│             │             │     │           │ Terminal  │
└─────────────┴─────────────┘     └───────────┴───────────┘
     2 windows                          3 windows
```

## Features

- **Zero configuration** - Just run the exe, windows auto-tile
- **BSP layout** - Binary Space Partitioning for balanced splits
- **Multi-monitor support** - Each monitor tiles independently
- **DPI-aware** - Correct positioning on high-DPI displays
- **DWM border compensation** - Pixel-perfect tiling
- **Lightweight** - ~1.1MB binary, event-driven, near-zero CPU at idle
- **No shortcuts** - No keyboard hooks, no conflicts with other apps

## Installation

### Download

1. Download `twm.exe` from the [Releases](https://github.com/kimkimjp/twm/releases) page
2. Run `twm.exe`

**Auto-start at login:** Press `Win+R`, type `shell:startup`, place a shortcut to `twm.exe` there.

**Run as Administrator** for managing elevated windows (e.g., Task Manager).

### Build from source

```bash
# Install Rust (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/kimkimjp/twm.git
cd twm
cargo build --release

# Cross-compile from Linux
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

## Configuration

Optional config file: `%APPDATA%\twm\config.yaml`

```yaml
gaps:
  inner: 5    # Gap between windows (pixels)
  outer: 10   # Gap between windows and screen edge (pixels)

window_rules:
  - class: "TaskManagerWindow"
    command: "floating"
  - title: "Calculator"
    command: "floating"
```

If no config file exists, twm uses sensible defaults (5px inner gap, 10px outer gap).

## Uninstallation

1. Close `twm.exe` (end task from Task Manager)
2. Delete `twm.exe`
3. Remove the shortcut from `shell:startup` (if configured)
4. Delete `%APPDATA%\twm\` (optional, removes config)

No registry entries. No system files modified.

## Architecture

```
twm.exe (single process, event-driven)
  |
  +-- Event Listener (SetWinEventHook + WINEVENT_OUTOFCONTEXT)
  |     Detects: window show, window destroy
  |
  +-- Window Manager Core
  |     +-- Monitor[] (auto-detected via EnumDisplayMonitors)
  |           +-- BSP Tree (auto-tiling layout engine)
  |
  +-- Win32 API Layer (DPI-aware, DWM border compensation)
```

## Requirements

- Windows 10 or later
- No runtime dependencies

## License

MIT
