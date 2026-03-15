# twm - Tiling Window Manager for Windows

A lightweight tiling window manager for Windows 10/11, inspired by [i3wm](https://i3wm.org/).

## Features

- **BSP (Binary Space Partitioning) layout** - Automatic tiling with horizontal/vertical splits
- **9 workspaces** - Switch between virtual workspaces with keyboard shortcuts
- **i3-compatible keybindings** - Familiar shortcuts for i3 users
- **YAML configuration** - Easy to customize
- **Lightweight** - ~1.2MB binary, minimal memory footprint, event-driven (near-zero CPU at idle)

## Default Keybindings

| Key | Action |
|-----|--------|
| `Alt+H/J/K/L` | Focus left/down/up/right |
| `Alt+Shift+H/J/K/L` | Move window left/down/up/right |
| `Alt+1-9` | Switch to workspace 1-9 |
| `Alt+Shift+1-9` | Move window to workspace 1-9 |
| `Alt+Shift+Q` | Close window |
| `Alt+F` | Toggle fullscreen |
| `Alt+V` | Toggle split direction |
| `Alt+Enter` | Open terminal (Windows Terminal) |
| `Alt+Shift+E` | Exit twm |
| `Alt+Shift+C` | Reload config |

## Configuration

Config file: `~/.config/twm/config.yaml`

```yaml
general:
  mod_key: "Alt"
  gaps:
    inner: 5
    outer: 10
  terminal: "wt.exe"

keybindings:
  - key: "Mod+H"
    command: "focus left"
  # ...

window_rules:
  - class: "TaskManagerWindow"
    command: "floating"
```

## Building

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build for Windows
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## License

MIT
