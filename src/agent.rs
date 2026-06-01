#![allow(dead_code)]

use crate::config::Config;
use crate::hint::{Hint, HintGrid};
use crate::pointer::{MouseButton, VirtualPointer};
use crate::render::{AppState, OutputInfo};

use std::cell::RefCell;
use std::rc::Rc;
use tracing::{error, info, warn};
use wayland_client::{Connection, QueueHandle};

#[derive(serde::Serialize)]
struct HintList {
    hints: Vec<Hint>,
}

pub struct AgentMode;

impl AgentMode {
    /// Retrieve display geometry and virtual pointer manager headlessly without windows
    fn setup_headless() -> anyhow::Result<(
        Connection,
        Rc<RefCell<AppState>>,
        QueueHandle<AppState>,
        i32,
        i32,
    )> {
        let conn = Connection::connect_to_env().map_err(|e| {
            error!("Could not connect to Wayland env: {:?}", e);
            anyhow::anyhow!("Wayland connection failed")
        })?;

        let mut event_queue = conn.new_event_queue();
        let qhandle = event_queue.handle();

        let state = Rc::new(RefCell::new(AppState::new()));

        // Setup registry binding
        let _registry = conn.display().get_registry(&qhandle, ());

        // Flush twice to discover outputs and populate resolution geometry
        event_queue.roundtrip(&mut *state.borrow_mut())?;
        event_queue.roundtrip(&mut *state.borrow_mut())?;

        let (width, height) = {
            let s = state.borrow();
            let out = s
                .outputs
                .iter()
                .filter_map(|(_, info)| info.clone())
                .next()
                .unwrap_or(OutputInfo {
                    name: "default".to_string(),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale: 1,
                });
            (out.width, out.height)
        };

        Ok((conn, state, qhandle, width, height))
    }

    /// Headlessly outputs the current hint grid coordinates in JSON format
    pub fn list_hints(config: &Config) -> anyhow::Result<()> {
        let (_conn, _state, _qhandle, width, height) = Self::setup_headless()?;

        let grid = HintGrid::generate_first_pass(width, height, &config.hint_chars, 0, false, None);
        let list = HintList { hints: grid.hints };

        let json_str = serde_json::to_string(&list)?;
        println!("{}", json_str);

        Ok(())
    }

    /// Headlessly warp cursor to matched label and trigger callback triggers
    pub fn select_hint(label: &str, config: &Config) -> anyhow::Result<()> {
        let (_conn, state, qhandle, width, height) = Self::setup_headless()?;

        let grid = HintGrid::generate_first_pass(width, height, &config.hint_chars, 0, false, None);
        let matched = grid.hints.iter().find(|h| h.label == label);

        if let Some(h) = matched {
            info!(
                "Headless select matched: label='{}' at ({}, {})",
                h.label, h.x, h.y
            );

            // Spawn virtual pointer and warp
            if let Some(ref manager) = state.borrow().virtual_pointer_manager {
                let pointer = VirtualPointer::new(manager, &qhandle);
                pointer.move_to(h.x, h.y, width, height);
                pointer.click(MouseButton::Left);
            } else {
                warn!("Virtual pointer manager protocol binding missing. Cannot simulate warp.");
            }

            // Execute select callback
            Config::execute_callback(&config.on_select_cmd, h.x, h.y, width, height)?;
        } else {
            error!(
                "Label '{:?}' did not match any active hint grid entries.",
                label
            );
            return Err(anyhow::anyhow!("Label mismatch"));
        }

        // Graceful exit callback spawner
        Config::execute_callback(&config.on_exit_cmd, 0, 0, width, height)?;

        Ok(())
    }

    /// Directly warping cursor to physical coordinates, optionally triggering button clicks
    pub fn move_to(
        x: i32,
        y: i32,
        click: Option<MouseButton>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let (_conn, state, qhandle, width, height) = Self::setup_headless()?;

        info!("Headless move_to resolved coordinates: ({}, {})", x, y);

        if let Some(ref manager) = state.borrow().virtual_pointer_manager {
            let pointer = VirtualPointer::new(manager, &qhandle);
            pointer.move_to(x, y, width, height);

            if let Some(btn) = click {
                pointer.click(btn);
            }
        } else {
            warn!("Virtual pointer manager protocol binding missing. Cannot simulate warp.");
        }

        // Trigger callbacks
        Config::execute_callback(&config.on_select_cmd, x, y, width, height)?;
        Config::execute_callback(&config.on_exit_cmd, 0, 0, width, height)?;

        Ok(())
    }
}
