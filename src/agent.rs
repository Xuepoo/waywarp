#![allow(dead_code)]

use crate::hint::HintGrid;

pub struct AgentMode;

impl AgentMode {
    pub fn list_hints_json(_grid: &HintGrid) -> String {
        // TODO: Return JSON representation of all active hints
        r#"{"hints":[]}"#.to_string()
    }
}
