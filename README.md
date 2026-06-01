# waywarp

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
waywarp --move 800 460 --click left
```

## Configuration

`~/.config/waywarp/config`:

```ini
on_select_cmd=hyprctl dispatch movecursor {x} {y} && ydotool click 0xC0
hint_font=JetBrainsMono Nerd Font
hint_size=22
```

## License

MIT
