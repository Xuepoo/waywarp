# waywarp

English | [简体中文](./README.zh-CN.md)

A high-performance, keyboard-driven mouse control tool for Wayland compositors (wlroots-based).

## Features

- **Hint-based cursor positioning**: Overlay hint labels on screen, type characters to warp the cursor
- **Multi-pass refinement**: Coarse grid → fine grid for pixel-perfect accuracy
- **Agent CLI mode**: JSON output, programmatic hint selection, direct coordinate control
- **Callback system**: Execute shell commands after positioning (click, type, drag, scroll)
- **Multi-monitor support**: Auto-detect monitors, per-monitor hint grids

## Supported Compositors

- Hyprland
- Sway
- River
- Wayfire
- niri
- Any wlroots-based compositor

## Installation

```bash
cargo install waywarp
```

Or build from source:

```bash
git clone https://github.com/Xuepoo/waywarp.git
cd waywarp
cargo build --release
```

## Usage

```bash
# Interactive hint mode
waywarp

# List hints as JSON
waywarp --list-hints --format json

# Programmatic hint selection
waywarp --select "fa"

# Direct coordinate + click
waywarp --move-to 800 460 --click left

# Relative coordinate move + click (v0.1.2+)
waywarp --move-by 50 -30 --click left

# Start in continuous Normal Mode (cursor drive) (v0.1.3+)
waywarp --normal
```

### Normal Mode Keybindings (Cursor Mode)

When starting with `waywarp --normal`, you enter a continuous keyboard-driven mouse pointer mode:

| Key | Action |
| --- | --- |
| `h` / `Left Arrow` | Move cursor left |
| `j` / `Down Arrow` | Move cursor down |
| `k` / `Up Arrow` | Move cursor up |
| `l` / `Right Arrow` | Move cursor right |
| `Shift` (hold) | 3x Speed Boost (Fast acceleration) |
| `Control` (hold) | 0.25x Speed Divisor (Fine precision) |
| `f` / `Return` | Left Click (Exits if `exit_on_select` is enabled) |
| `d` | Right Click (Exits if `exit_on_select` is enabled) |
| `s` | Middle Click |
| `u` | Scroll Up |
| `e` | Scroll Down |
| `Escape` / `q` | Exit Normal Mode |

## Configuration

Waywarp automatically creates a default configuration file at `~/.config/waywarp/config` on its first run if it does not exist.

```ini
# ~/.config/waywarp/config
on_select_cmd=hyprctl dispatch movecursor {global_x} {global_y}
hint_font=monospace
hint_size=18
```

For the complete documentation of all configuration options, callbacks, dynamic variables, and integration recipes, refer to the [Waywarp Configuration Guide](docs/configuration.md).

## License

MIT
