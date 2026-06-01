# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.3] - 2026-06-02

### Added
- **Continuous Keyboard Normal Mode (Cursor Mode)**: Implemented continuous cursor sliding using `hjkl` or arrow keys with sub-16ms tick rate (~60Hz) for smooth movements.
- **Dynamic Speed & Acceleration**: Integrated keyboard modifiers (`Shift` for 3x speed boost, `Control` for 0.25x precision reduction) along with dynamic cursor acceleration over time.
- **Simulated Mouse Actions**: Added keysym mappings in Normal Mode for Left Click (`f` or `Return`), Right Click (`d`), Middle Click (`s`), Scroll Up (`u`), and Scroll Down (`e`).
- **Command Line Mode Integration**: Added a `--normal` boolean CLI argument to start Waywarp directly in Normal Mode overlay.
- **Customizable Normal Mode Keybindings**: Enabled full keybinding customization inside `~/.config/waywarp/config` (supporting comma-separated fallback lists like `key_left=h,Left`). Parses standard XKB keysym names dynamically at runtime.

---

## [0.1.2] - 2026-06-01

### Added
- **Relative Cursor Movement**: Introduced the new `--move-by <dx> <dy>` CLI argument to enable relative mouse movement based on current positions. Useful for precise manual adjustments, scripting, and relative AI click automation.
- **Simplified Chinese Translation**: Provided a full-featured `README.zh-CN.md` to support the mainland Chinese Wayland community.
- **Dynamic Configuration Documentation**: Added a comprehensive `docs/configuration.md` configuration spec covering INI keys, callbacks, dynamic variables, and custom recipes.

### Fixed
- **Startup Warnings Silencing**: Corrected the configuration loader sequence in `config.rs`. First-time startups without existing config files will now quietly auto-generate the defaults and log a friendly `info!` confirmation, eliminating scary/unnecessary `WARN` logs.

---

## [0.1.1] - 2026-06-01

### Added
- **Multi-architecture Native Packaging**: Expanded the GitHub Actions build matrix to target both `x86_64` (standard PC) and `aarch64` (ARM64) Linux architectures natively using native hardware runners.
- **Arch Linux Package Support**: Added Arch Linux native `.pkg.tar.zst` package generation for both architectures via nFPM's `archlinux` packager.

### Fixed
- **Crates.io Publish Guard**: Integrated an idempotent API check utilizing a custom User-Agent to safely bypass redundant `cargo publish` runs on tag re-releases, resolving exit code `101` pipeline failures.
- **nFPM Pathing**: Explicitly linked `packaging/nfpm.yaml` to resolve relative path lookup errors during headless root-level packaging runs.

---

## [0.1.0] - 2026-06-01

### Added
- **Interactive Grid Overlay**: Transparent full-screen layout overlays displaying visual grid keys.
- **Multi-pass Refinement**: Coarse pass for regions, fine pass for zooming and pixel-perfect positioning.
- **Agent CLI Integration**: Structured JSON list-hints endpoints (`--list-hints`) and programmatic selection (`--select`).
- **Callbacks System**: Extensible INI configuration spawner executing arbitrary post-select and post-exit shell scripts with dynamic variables.
