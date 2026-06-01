# Waywarp Configuration Guide

Waywarp loads its configuration from a simple flat INI file located at `~/.config/waywarp/config` (or `$XDG_CONFIG_HOME/waywarp/config`). If the configuration directory or file is missing on the first launch, Waywarp will automatically generate a default configuration file.

---

## Configuration File Location

* **Standard Path**: `~/.config/waywarp/config`
* **XDG Compliant**: If `$XDG_CONFIG_HOME` is set, it will be saved to `$XDG_CONFIG_HOME/waywarp/config`.

---

## Configuration Options

Below is a detailed breakdown of all supported keys in the configuration file, along with their default values and descriptions:

| Key | Default Value | Description |
| :--- | :--- | :--- |
| `hint_bg` | `#ff555560` | Translucent background color of the hint overlays in 8-digit Hex RGBA format (`#RRGGBBAA`). |
| `hint_fg` | `#ffffffff` | Text color of the hint overlays in 8-digit Hex RGBA format (`#RRGGBBAA`). |
| `hint_font` | `monospace` | Pango-compatible font family name (e.g., `sans-serif`, `JetBrainsMono Nerd Font`, `Inter`). |
| `hint_size` | `18` | Font size in points. |
| `hint_border_radius` | `25.0` | Corner rounding radius of the hint overlay blocks (set to `0.0` for sharp rectangular borders). |
| `hint_chars` | `asdfghjklqwertzxv` | The key character set used for generating hint labels. Must consist of unique lowercase letters. |
| `refinement_passes` | `2` | Number of grid passes for positioning. `1` warp directly to key. `2` coarse select → fine refine (recommended). |
| `exit_on_select` | `true` | Automatically close the hint overlays and exit the program once a final selection is made. |
| `on_select_cmd` | *See below* | Shell callback executed immediately after warping. Supports dynamic variable placeholders. |
| `on_exit_cmd` | *(Empty)* | Shell callback executed when the user manually cancels or exits without warping. |

---

## Callback Shell Variables

The `on_select_cmd` command string supports several useful dynamic variable placeholders that are replaced at runtime before execution:

* `{global_x}` - Absolute X coordinate of the cursor across all monitors.
* `{global_y}` - Absolute Y coordinate of the cursor across all monitors.
* `{x}` - Relative X coordinate inside the targeted monitor bounds.
* `{y}` - Relative Y coordinate inside the targeted monitor bounds.
* `{monitor_id}` - The index or name of the monitor selected.

---

## Practical Examples

Here are some standard, highly optimized configuration recipes for different compositor architectures and workflows:

### 1. Hyprland Integration (Default)

Use Hyprland's native dispatcher to warp the cursor, then trigger a mouse click using `ydotool`:

```ini
# ~/.config/waywarp/config
hint_bg=#3b4252cc
hint_fg=#eceff4ff
hint_font=JetBrainsMono Nerd Font
hint_size=20
hint_border_radius=8.0
refinement_passes=2

# Warp the cursor natively in Hyprland, then simulate a left click using ydotool
on_select_cmd=hyprctl dispatch movecursor {global_x} {global_y} && ydotool click 0xC0
```

### 2. Generic wlroots / Sway (using wlrctl and ydotool)

For compositors that do not have custom IPC warp commands, use `wlrctl` or virtual pointer warp, and trigger actions:

```ini
# ~/.config/waywarp/config
hint_bg=#1e1e2ecc
hint_fg=#cdd6f4ff
hint_font=Outfit
hint_size=22
hint_border_radius=12.0
refinement_passes=2

# Warp and trigger a right-click
on_select_cmd=wlrctl pointer move {global_x} {global_y} && ydotool click 0xC1
```

### 3. Non-Interactive Agent CLI clicks

If you are calling Waywarp programmatically from AI Agents or terminal scripts, you can pass custom commands or let it print the coordinates natively for capture:

```ini
# ~/.config/waywarp/config
# Disable automatic exit to allow chaining or manual control
exit_on_select=false
on_select_cmd=echo "Warped to X:{global_x} Y:{global_y}"
```
