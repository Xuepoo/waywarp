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
    pub is_element_based: bool,
}

impl HintGrid {
    pub fn new(width: i32, height: i32, screen: u32) -> Self {
        Self {
            hints: Vec::new(),
            width,
            height,
            screen,
            is_element_based: false,
        }
    }

    /// Generate prefix-free fixed-length labels dynamically using Base-M representation.
    /// Ensures that no label is a prefix of another by padding labels to a uniform length.
    pub fn generate_labels(count: usize, chars: &[char]) -> Vec<String> {
        if count == 0 {
            return Vec::new();
        }
        let base = chars.len();
        if base == 0 {
            return Vec::new();
        }
        if base == 1 && count > 1 {
            return Vec::new();
        }

        let mut length = 1;
        let mut capacity = base;
        while capacity < count {
            length += 1;
            capacity = capacity.saturating_mul(base);
            if capacity == usize::MAX {
                break;
            }
        }

        let mut labels = Vec::with_capacity(count);
        for i in 0..count {
            let mut label = String::new();
            let mut temp = i;
            for _ in 0..length {
                let digit = temp % base;
                label.push(chars[digit]);
                temp /= base;
            }
            labels.push(label.chars().rev().collect());
        }
        labels
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_unique_chars() {
        let chars = HintGrid::get_unique_chars("aabcc123!d");
        assert_eq!(chars, vec!['a', 'b', 'c', 'd']);

        let empty = HintGrid::get_unique_chars("123!");
        assert!(!empty.is_empty()); // Falls back to default chars
    }

    #[test]
    fn test_generate_first_pass_single_monitor() {
        let hint_chars = "abc";
        let grid = HintGrid::generate_first_pass(1920, 1080, hint_chars, 0, false, None);

        // 3^2 = 9 hints
        assert_eq!(grid.hints.len(), 9);
        assert_eq!(grid.hints[0].label, "aa");
        assert_eq!(grid.hints[1].label, "ab");
        assert_eq!(grid.hints[8].label, "cc");
        assert_eq!(grid.hints[0].screen, 0);

        // Bounds check coordinates
        for hint in &grid.hints {
            assert!(hint.x >= 0 && hint.x <= 1920);
            assert!(hint.y >= 0 && hint.y <= 1080);
        }
    }

    #[test]
    fn test_generate_first_pass_multi_monitor() {
        let hint_chars = "abc";
        let grid = HintGrid::generate_first_pass(1920, 1080, hint_chars, 1, true, Some('s'));

        // 3^2 = 9 hints
        assert_eq!(grid.hints.len(), 9);
        // Multi-monitor tags should have length of 3 starting with monitor char
        assert_eq!(grid.hints[0].label, "saa");
        assert_eq!(grid.hints[1].label, "sab");
        assert_eq!(grid.hints[8].label, "scc");
        assert_eq!(grid.hints[0].screen, 1);
    }

    #[test]
    fn test_generate_refinement() {
        let hint_chars = "ab";
        let grid = HintGrid::generate_refinement(100, 200, 300, 400, hint_chars, 0);

        // 2^2 = 4 hints
        assert_eq!(grid.hints.len(), 4);
        assert_eq!(grid.hints[0].label, "aa");
        assert_eq!(grid.hints[3].label, "bb");

        // First cell midpoint should be (100 + 100/2, 200 + 100/2) => (150, 250)
        assert_eq!(grid.hints[0].x, 150);
        assert_eq!(grid.hints[0].y, 250);
    }

    #[test]
    fn test_generate_labels_prefix_free() {
        let chars = vec!['a', 'b', 'c']; // base 3
        let labels = HintGrid::generate_labels(5, &chars);
        // With 5 elements and base 3, minimal fixed length L = ceil(log_3(5)) = 2.
        assert_eq!(labels.len(), 5);
        for label in &labels {
            assert_eq!(label.len(), 2);
        }
        // Prefix-free verification: no label is a prefix of another
        for i in 0..labels.len() {
            for j in 0..labels.len() {
                if i != j {
                    assert!(!labels[i].starts_with(&labels[j]));
                }
            }
        }
    }
}
