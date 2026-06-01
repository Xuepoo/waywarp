#![allow(dead_code)]

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Default modern waywarp configuration path under ~/.config
const CONFIG_SUBDIR: &str = "waywarp";
const CONFIG_FILE_NAME: &str = "config";

#[derive(Debug, Clone)]
pub struct Config {
    pub hint_bg: [f64; 4],       // RGBA, values from 0.0 to 1.0
    pub hint_fg: [f64; 4],       // RGBA, values from 0.0 to 1.0
    pub hint_font: String,       // Font family name
    pub hint_size: u32,          // Font size
    pub hint_border_radius: f64, // Corner radius
    pub hint_chars: String,      // Key character set
    pub refinement_passes: u32,  // Multi-pass refinement layers
    pub exit_on_select: bool,    // Close automatically after warp
    pub on_select_cmd: String,   // Command callback post-warp
    pub on_exit_cmd: String,     // Command callback on exit
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hint_bg: [1.0, 0.33, 0.33, 0.38], // #ff555560
            hint_fg: [1.0, 1.0, 1.0, 1.0],    // #ffffffff
            hint_font: "monospace".to_string(),
            hint_size: 18,
            hint_border_radius: 25.0,
            hint_chars: "asdfghjklqwertzxv".to_string(),
            refinement_passes: 2,
            exit_on_select: true,
            on_select_cmd: "hyprctl dispatch movecursor {global_x} {global_y}".to_string(),
            on_exit_cmd: "".to_string(),
        }
    }
}

impl Config {
    /// Retrieve default config file destination (XDG_CONFIG_HOME or ~/.config)
    pub fn get_config_path() -> PathBuf {
        let base_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".config")
            });
        base_dir.join(CONFIG_SUBDIR).join(CONFIG_FILE_NAME)
    }

    /// Load config from file. Creates default if file is missing.
    pub fn load() -> Self {
        let path = Self::get_config_path();
        if !path.exists() {
            // Try to create the default config file
            if let Err(write_err) = Self::write_default_config(&path) {
                warn!(
                    "Could not write default config to {:?}: {}",
                    path, write_err
                );
            } else {
                info!(
                    "Successfully created default configuration file at {:?}",
                    path
                );
            }
            return Config::default();
        }

        match Self::load_from_path(&path) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(
                    "Failed to load configuration from {:?}: {}. Falling back to defaults.",
                    path, e
                );
                Config::default()
            }
        }
    }

    /// Explicitly load from a specific file path (for custom tests / CLI overrides)
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Err(anyhow::anyhow!("Config file does not exist"));
        }

        let content = fs::read_to_string(path)?;
        Self::parse_ini(&content)
    }

    /// Write the standard default configuration file to disk
    fn write_default_config(path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;
        writeln!(file, "# waywarp config")?;
        writeln!(file, "hint_bg=#ff555560")?;
        writeln!(file, "hint_fg=#ffffffff")?;
        writeln!(file, "hint_font=monospace")?;
        writeln!(file, "hint_size=18")?;
        writeln!(file, "hint_border_radius=25.0")?;
        writeln!(file, "hint_chars=asdfghjklqwertzxv")?;
        writeln!(file, "refinement_passes=2")?;
        writeln!(file, "exit_on_select=true")?;
        writeln!(
            file,
            "on_select_cmd=hyprctl dispatch movecursor {{global_x}} {{global_y}}"
        )?;
        writeln!(file, "on_exit_cmd=")?;
        Ok(())
    }

    /// Parse flat INI string line-by-line
    fn parse_ini(content: &str) -> anyhow::Result<Self> {
        let mut config = Config::default();

        for (line_num, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim();
            let val = parts.next().unwrap_or("").trim();

            if key.is_empty() {
                continue;
            }

            // Support both modern naming and hyprwarp backward compatibility
            match key {
                "hint_bg" | "hint_bgcolor" => {
                    if let Some(rgba) = parse_hex_color(val) {
                        config.hint_bg = rgba;
                    } else {
                        warn!(
                            "Line {}: invalid color format for {}, using default",
                            line_num + 1,
                            key
                        );
                    }
                }
                "hint_fg" | "hint_fgcolor" => {
                    if let Some(rgba) = parse_hex_color(val) {
                        config.hint_fg = rgba;
                    } else {
                        warn!(
                            "Line {}: invalid color format for {}, using default",
                            line_num + 1,
                            key
                        );
                    }
                }
                "hint_font" => {
                    config.hint_font = val.to_string();
                }
                "hint_size" => {
                    if let Ok(size) = val.parse::<u32>() {
                        config.hint_size = size.clamp(8, 64);
                    }
                }
                "hint_border_radius" | "hint_radius" => {
                    if let Ok(radius) = val.parse::<f64>() {
                        config.hint_border_radius = radius.clamp(0.0, 100.0);
                    }
                }
                "hint_chars" => {
                    if !val.is_empty() {
                        config.hint_chars = val.to_string();
                    }
                }
                "refinement_passes" => {
                    if let Ok(passes) = val.parse::<u32>() {
                        config.refinement_passes = passes.clamp(1, 4);
                    }
                }
                "exit_on_select" => {
                    if let Ok(b) = val.parse::<bool>() {
                        config.exit_on_select = b;
                    }
                }
                "on_select_cmd" => {
                    config.on_select_cmd = val.to_string();
                }
                "on_exit_cmd" => {
                    config.on_exit_cmd = val.to_string();
                }
                _ => {
                    warn!("Line {}: unknown configuration key '{}'", line_num + 1, key);
                }
            }
        }

        Ok(config)
    }

    /// Spawns the specified shell command, replacing coordinate/scale placeholders
    pub fn execute_callback(
        cmd_template: &str,
        x: i32,
        y: i32,
        screen_w: i32,
        screen_h: i32,
    ) -> anyhow::Result<()> {
        if cmd_template.trim().is_empty() {
            return Ok(());
        }

        let scale_x = x as f64 / screen_w as f64;
        let scale_y = y as f64 / screen_h as f64;

        let formatted = cmd_template
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
            .replace("{global_x}", &x.to_string())
            .replace("{global_y}", &y.to_string())
            .replace("{screen_w}", &screen_w.to_string())
            .replace("{screen_h}", &screen_h.to_string())
            .replace("{scale_x}", &format!("{:.4}", scale_x))
            .replace("{scale_y}", &format!("{:.4}", scale_y));

        info!("Spawning action callback command: {:?}", formatted);

        std::process::Command::new("sh")
            .arg("-c")
            .arg(&formatted)
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn shell callback command: {:?}", e);
                e
            })?;

        Ok(())
    }
}

/// Helper function to parse hex color (#RRGGBBAA, #RRGGBB, RRGGBBAA, RRGGBB) to [f64; 4]
fn parse_hex_color(hex: &str) -> Option<[f64; 4]> {
    let clean = hex.trim().trim_start_matches('#');
    let len = clean.len();
    if len == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()? as f64 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()? as f64 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()? as f64 / 255.0;
        Some([r, g, b, 1.0])
    } else if len == 8 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()? as f64 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()? as f64 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()? as f64 / 255.0;
        let a = u8::from_str_radix(&clean[6..8], 16).ok()? as f64 / 255.0;
        Some([r, g, b, a])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ffffff"), Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(parse_hex_color("ffffff"), Some([1.0, 1.0, 1.0, 1.0]));

        let red_translucent = parse_hex_color("#ff000080").unwrap();
        assert!((red_translucent[0] - 1.0).abs() < 1e-5);
        assert!((red_translucent[1] - 0.0).abs() < 1e-5);
        assert!((red_translucent[2] - 0.0).abs() < 1e-5);
        assert!((red_translucent[3] - 128.0 / 255.0).abs() < 1e-5);

        assert_eq!(parse_hex_color("invalid"), None);
    }

    #[test]
    fn test_parse_ini_basic() {
        let ini = r#"
            # comment
            hint_bg = #11223344
            hint_fg = #556677
            hint_font = JetBrainsMono
            hint_size = 20
            hint_border_radius = 5.0
            hint_chars = abc
            refinement_passes = 3
            exit_on_select = false
            on_select_cmd = echo selected
            on_exit_cmd = echo exited
        "#;

        let cfg = Config::parse_ini(ini).unwrap();
        assert!((cfg.hint_bg[0] - 17.0 / 255.0).abs() < 1e-5);
        assert!((cfg.hint_bg[3] - 68.0 / 255.0).abs() < 1e-5);
        assert_eq!(cfg.hint_font, "JetBrainsMono");
        assert_eq!(cfg.hint_size, 20);
        assert_eq!(cfg.hint_border_radius, 5.0);
        assert_eq!(cfg.hint_chars, "abc");
        assert_eq!(cfg.refinement_passes, 3);
        assert!(!cfg.exit_on_select);
        assert_eq!(cfg.on_select_cmd, "echo selected");
        assert_eq!(cfg.on_exit_cmd, "echo exited");
    }

    #[test]
    fn test_parse_ini_compatibility() {
        let ini = r#"
            hint_bgcolor = #ff0000
            hint_fgcolor = #00ff00
            hint_radius = 12
        "#;

        let cfg = Config::parse_ini(ini).unwrap();
        assert!((cfg.hint_bg[0] - 1.0).abs() < 1e-5);
        assert!((cfg.hint_fg[1] - 1.0).abs() < 1e-5);
        assert_eq!(cfg.hint_border_radius, 12.0);
    }

    #[test]
    fn test_parse_ini_clamps() {
        let ini = r#"
            hint_size = 100
            hint_border_radius = 200
            refinement_passes = 10
        "#;

        let cfg = Config::parse_ini(ini).unwrap();
        assert_eq!(cfg.hint_size, 64); // Clamped to 64
        assert_eq!(cfg.hint_border_radius, 100.0); // Clamped to 100.0
        assert_eq!(cfg.refinement_passes, 4); // Clamped to 4
    }
}
