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
