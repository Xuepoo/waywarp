# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
