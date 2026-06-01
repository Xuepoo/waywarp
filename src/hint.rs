#![allow(dead_code)]

/// Represents a single hint tag on the screen
#[derive(Debug, Clone, serde::Serialize)]
pub struct Hint {
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub screen: u32,
}

/// Grid representation
pub struct HintGrid {
    pub hints: Vec<Hint>,
    pub width: i32,
    pub height: i32,
    pub screen: u32,
}

impl HintGrid {
    pub fn new(width: i32, height: i32, screen: u32) -> Self {
        Self {
            hints: Vec::new(),
            width,
            height,
            screen,
        }
    }

    /// Helper to get unique, ordered characters from config string
    pub fn get_unique_chars(hint_chars: &str) -> Vec<char> {
        let mut unique = Vec::new();
        for c in hint_chars.chars() {
            if c.is_alphabetic() && !unique.contains(&c) {
                unique.push(c);
            }
        }
        if unique.is_empty() {
            // Safe fallback if config is completely empty/corrupted
            unique = "asdfghjklqwertzxv".chars().collect();
        }
        unique
    }

    /// Generate initial grid spanning full monitor boundaries
    pub fn generate_first_pass(
        width: i32,
        height: i32,
        hint_chars: &str,
        screen: u32,
        multi_monitor: bool,
        monitor_char: Option<char>,
    ) -> Self {
        let mut grid = Self::new(width, height, screen);
        let chars = Self::get_unique_chars(hint_chars);
        let u = chars.len();

        let cell_w = width / u as i32;
        let cell_h = height / u as i32;

        let hint_w = 40;
        let hint_h = 30;

        for i in 0..u {
            for j in 0..u {
                let cell_x = i as i32 * cell_w;
                let cell_y = j as i32 * cell_h;

                // Center of cell
                let x = cell_x + cell_w / 2;
                let y = cell_y + cell_h / 2;

                let label = if multi_monitor {
                    let m_char = monitor_char.unwrap_or(chars[screen as usize % u]);
                    format!("{}{}{}", m_char, chars[i], chars[j])
                } else {
                    format!("{}{}", chars[i], chars[j])
                };

                grid.hints.push(Hint {
                    label,
                    x,
                    y,
                    width: hint_w,
                    height: hint_h,
                    screen,
                });
            }
        }

        grid
    }

    /// Subdivide a specific grid region for micro-refinement
    pub fn generate_refinement(
        x_min: i32,
        y_min: i32,
        x_max: i32,
        y_max: i32,
        hint_chars: &str,
        screen: u32,
    ) -> Self {
        let width = x_max - x_min;
        let height = y_max - y_min;
        let mut grid = Self::new(width, height, screen);

        let chars = Self::get_unique_chars(hint_chars);
        let u = chars.len();

        let cell_w = width / u as i32;
        let cell_h = height / u as i32;

        let hint_w = 40;
        let hint_h = 30;

        for i in 0..u {
            for j in 0..u {
                let cell_x = x_min + i as i32 * cell_w;
                let cell_y = y_min + j as i32 * cell_h;

                let x = cell_x + cell_w / 2;
                let y = cell_y + cell_h / 2;

                let label = format!("{}{}", chars[i], chars[j]);

                grid.hints.push(Hint {
                    label,
                    x,
                    y,
                    width: hint_w,
                    height: hint_h,
                    screen,
                });
            }
        }

        grid
    }
}
