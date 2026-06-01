#![allow(dead_code)]

use crate::config::Config;
use crate::hint::HintGrid;

pub struct Renderer {
    // Wayland & Cairo specific state will go here
}

impl Renderer {
    pub fn new() -> anyhow::Result<Self> {
        // TODO: Initialize Wayland connections
        Ok(Self {})
    }

    pub fn draw_overlay(&mut self, _grid: &HintGrid, _config: &Config) -> anyhow::Result<()> {
        // TODO: Construct layer shell surfaces and draw hints via Cairo
        Ok(())
    }
}
