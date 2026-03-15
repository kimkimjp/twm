# twm - Tiling Window Manager for Windows

A lightweight tiling window manager for Windows 10/11, inspired by [i3wm](https://i3wm.org/).

## Features

- **BSP (Binary Space Partitioning) layout** - Automatic tiling with horizontal/vertical splits
- **Multi-monitor support** - Each monitor has independent workspaces
- **9 workspaces per monitor** - Switch between virtual workspaces with keyboard shortcuts
- **i3-compatible keybindings** - Familiar shortcuts for i3 users
- **YAML configuration** - Easy to customize keybindings, gaps, and window rules
- **Low-level keyboard hook** - Intercepts shortcuts before Windows processes them (no conflicts with OS shortcuts)
- **DPI-aware** - Correct window positioning on high-DPI displays
- **DWM border compensation** - Pixel-perfect tiling with no gaps between windows
- **Lightweight** - ~1.2MB binary, minimal memory footprint, event-driven (near-zero CPU at idle)
- **Window rules** - Auto-float specific windows by class name or title

## Installation

### Option 1: Download the binary

1. Download `twm.exe` from the [Releases](https://github.com/kimkimjp/twm/releases) page
2. Place it anywhere on your system (e.g., `C:\Program Files\twm\twm.exe`)
3. (Optional) Create a config file at `%APPDATA%\twm\config.yaml` (see [Configuration](#configuration))
4. Run `twm.exe`

**Tip:** To start twm automatically at login, create a shortcut to `twm.exe` in `shell:startup` (press `Win+R`, type `shell:startup`, and place the shortcut there).

**Tip:** Running as Administrator allows twm to manage elevated windows (e.g., Task Manager). Right-click the shortcut > Properties > Advanced > "Run as administrator".

### Option 2: Build from source

```bash
# Install Rust (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/kimkimjp/twm.git
cd twm

# Build (native on Windows)
cargo build --release

# Or cross-compile from Linux
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

The binary will be at `target/release/twm.exe` (or `target/x86_64-pc-windows-gnu/release/twm.exe` for cross-compilation).

## Uninstallation

twm is a single portable executable with no installer. To uninstall:

1. **Stop twm** - Press `Alt+Shift+E` to exit, or end `twm.exe` from Task Manager
2. **Delete the executable** - Remove `twm.exe` from wherever you placed it
3. **Remove auto-start** (if configured) - Press `Win+R`, type `shell:startup`, and delete the twm shortcut
4. **Remove config** (optional) - Delete the folder `%APPDATA%\twm\` (typically `C:\Users\<name>\AppData\Roaming\twm\`)

No registry entries are created. No system files are modified.

## Default Keybindings

`Mod` = `Alt` key by default.

### Window Navigation

| Key | Action |
|-----|--------|
| `Mod+H` | Focus left |
| `Mod+J` | Focus down |
| `Mod+K` | Focus up |
| `Mod+L` | Focus right |

### Window Movement

| Key | Action |
|-----|--------|
| `Mod+Shift+H` | Move window left |
| `Mod+Shift+J` | Move window down |
| `Mod+Shift+K` | Move window up |
| `Mod+Shift+L` | Move window right |

### Workspaces

| Key | Action |
|-----|--------|
| `Mod+1-9` | Switch to workspace 1-9 |
| `Mod+Shift+1-9` | Move window to workspace 1-9 |

### Multi-Monitor

| Key | Action |
|-----|--------|
| `Mod+.` | Focus next monitor |
| `Mod+,` | Focus previous monitor |
| `Mod+Shift+.` | Move window to next monitor |
| `Mod+Shift+,` | Move window to previous monitor |

### Window Operations

| Key | Action |
|-----|--------|
| `Mod+Shift+Q` | Close window |
| `Mod+F` | Toggle fullscreen |
| `Mod+V` | Toggle split direction (horizontal/vertical) |
| `Mod+Enter` | Open terminal (Windows Terminal) |

### System

| Key | Action |
|-----|--------|
| `Mod+Shift+C` | Reload config |
| `Mod+Shift+E` | Exit twm |

## Configuration

Config file location: `%APPDATA%\twm\config.yaml` (typically `C:\Users\<name>\AppData\Roaming\twm\config.yaml`)

If no config file exists, twm uses sensible defaults.

```yaml
general:
  mod_key: "Alt"          # Modifier key: "Alt" or "Super"
  gaps:
    inner: 5              # Gap between windows (pixels)
    outer: 10             # Gap between windows and screen edge (pixels)
  terminal: "wt.exe"      # Terminal launched by Mod+Enter

keybindings:
  - key: "Mod+H"
    command: "focus left"
  - key: "Mod+J"
    command: "focus down"
  - key: "Mod+K"
    command: "focus up"
  - key: "Mod+L"
    command: "focus right"
  - key: "Mod+Shift+H"
    command: "move left"
  - key: "Mod+Shift+J"
    command: "move down"
  - key: "Mod+Shift+K"
    command: "move up"
  - key: "Mod+Shift+L"
    command: "move right"
  - key: "Mod+1"
    command: "workspace 1"
  # ... Mod+2 through Mod+9
  - key: "Mod+Shift+1"
    command: "move_to_workspace 1"
  # ... Mod+Shift+2 through Mod+Shift+9
  - key: "Mod+Shift+Q"
    command: "close"
  - key: "Mod+F"
    command: "fullscreen"
  - key: "Mod+V"
    command: "split_toggle"
  - key: "Mod+Return"
    command: "exec wt.exe"
  - key: "Mod+Period"
    command: "focus_monitor next"
  - key: "Mod+Comma"
    command: "focus_monitor prev"
  - key: "Mod+Shift+Period"
    command: "move_to_monitor next"
  - key: "Mod+Shift+Comma"
    command: "move_to_monitor prev"
  - key: "Mod+Shift+E"
    command: "exit"
  - key: "Mod+Shift+C"
    command: "reload"

window_rules:
  - class: "TaskManagerWindow"
    command: "floating"
  - title: "Calculator"
    command: "floating"
```

### Available Commands

| Command | Description |
|---------|-------------|
| `focus left/right/up/down` | Move focus in direction |
| `move left/right/up/down` | Move window in direction |
| `workspace 1-9` | Switch to workspace |
| `move_to_workspace 1-9` | Move window to workspace |
| `focus_monitor next/prev` | Focus next/previous monitor |
| `move_to_monitor next/prev` | Move window to next/previous monitor |
| `close` | Close focused window |
| `fullscreen` | Toggle fullscreen |
| `split_toggle` | Toggle split direction |
| `exec <command>` | Execute a shell command |
| `reload` | Reload configuration |
| `exit` | Exit twm |

## Architecture

```
twm.exe (single process, event-driven)
  |
  +-- Low-Level Keyboard Hook (WH_KEYBOARD_LL via SetWindowsHookEx)
  |     Intercepts ALL keyboard input before Windows processes it.
  |     Matched shortcuts are consumed; unmatched keys pass through.
  |
  +-- Event Listener (SetWinEventHook + WINEVENT_OUTOFCONTEXT)
  +-- Window Manager Core
  |     +-- Monitor[] (auto-detected via EnumDisplayMonitors)
  |           +-- Workspace[9] (per monitor)
  |                 +-- BSP Tree (layout engine)
  +-- Config Manager (YAML)
  +-- Win32 API Layer (DPI-aware, DWM border compensation)
```

## Requirements

- Windows 10 or later
- No runtime dependencies (single static binary)

## License

MIT
