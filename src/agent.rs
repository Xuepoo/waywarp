#![allow(dead_code)]

use crate::config::Config;
use crate::hint::{Hint, HintGrid};
use crate::pointer::{MouseButton, VirtualPointer};
use crate::render::{AppState, OutputInfo};

use std::cell::RefCell;
use std::rc::Rc;
use tracing::{error, info, warn};
use wayland_client::{Connection, QueueHandle, protocol::wl_output};

#[derive(serde::Serialize)]
struct HintList {
    hints: Vec<Hint>,
}

pub struct AgentMode;

impl AgentMode {
    /// Retrieve display geometry and virtual pointer manager headlessly without windows
    #[allow(clippy::type_complexity)]
    fn setup_headless() -> anyhow::Result<(
        Connection,
        Rc<RefCell<AppState>>,
        QueueHandle<AppState>,
        Vec<(wl_output::WlOutput, OutputInfo)>,
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

        let active_outputs: Vec<(wl_output::WlOutput, OutputInfo)> = {
            let s = state.borrow();
            s.outputs
                .iter()
                .filter_map(|(o, info)| info.clone().map(|i| (o.clone(), i)))
                .filter(|(_, info)| info.width > 0 && info.height > 0)
                .collect()
        };

        let active_outputs = if active_outputs.is_empty() {
            let fallback_output = {
                let s = state.borrow();
                s.outputs
                    .first()
                    .map(|(o, _)| o.clone())
                    .ok_or_else(|| anyhow::anyhow!("No outputs registered at all"))?
            };
            vec![(
                fallback_output,
                OutputInfo {
                    name: "default".to_string(),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale: 1,
                },
            )]
        } else {
            active_outputs
        };

        Ok((conn, state, qhandle, active_outputs))
    }

    /// Headlessly outputs the current hint grid coordinates in JSON format
    pub fn list_hints(config: &Config) -> anyhow::Result<()> {
        let (_conn, _state, _qhandle, active_outputs) = Self::setup_headless()?;
        let is_multi = active_outputs.len() > 1;
        let chars = HintGrid::get_unique_chars(&config.hint_chars);

        let mut all_hints = Vec::new();
        if is_multi {
            for (i, (_, info)) in active_outputs.iter().enumerate() {
                let monitor_char = chars[i % chars.len()];
                let grid = HintGrid::generate_first_pass(
                    info.width,
                    info.height,
                    &config.hint_chars,
                    i as u32,
                    true,
                    Some(monitor_char),
                );
                all_hints.extend(grid.hints);
            }
        } else {
            let (_, info) = &active_outputs[0];
            let grid = HintGrid::generate_first_pass(
                info.width,
                info.height,
                &config.hint_chars,
                0,
                false,
                None,
            );
            all_hints.extend(grid.hints);
        }

        let list = HintList { hints: all_hints };
        let json_str = serde_json::to_string(&list)?;
        println!("{}", json_str);

        Ok(())
    }

    /// Headlessly warp cursor to matched label and trigger callback triggers
    pub fn select_hint(label: &str, config: &Config) -> anyhow::Result<(i32, i32, u32)> {
        let (_conn, state, qhandle, active_outputs) = Self::setup_headless()?;
        let is_multi = active_outputs.len() > 1;
        let chars = HintGrid::get_unique_chars(&config.hint_chars);

        let mut all_hints = Vec::new();
        if is_multi {
            for (i, (_, info)) in active_outputs.iter().enumerate() {
                let monitor_char = chars[i % chars.len()];
                let grid = HintGrid::generate_first_pass(
                    info.width,
                    info.height,
                    &config.hint_chars,
                    i as u32,
                    true,
                    Some(monitor_char),
                );
                all_hints.extend(grid.hints);
            }
        } else {
            let (_, info) = &active_outputs[0];
            let grid = HintGrid::generate_first_pass(
                info.width,
                info.height,
                &config.hint_chars,
                0,
                false,
                None,
            );
            all_hints.extend(grid.hints);
        }

        let matched = all_hints.iter().find(|h| h.label == label);

        if let Some(h) = matched {
            info!(
                "Headless select matched: label='{}' at ({}, {}) on screen {}",
                h.label, h.x, h.y, h.screen
            );

            let (target_output, target_info) = active_outputs
                .iter()
                .enumerate()
                .find(|(idx, _)| *idx as u32 == h.screen)
                .map(|(_, (o, info))| (Some(o), info.clone()))
                .unwrap_or((
                    None,
                    OutputInfo {
                        name: "default".to_string(),
                        x: 0,
                        y: 0,
                        width: 1920,
                        height: 1080,
                        scale: 1,
                    },
                ));

            // Spawn virtual pointer and warp
            if let Some(ref manager) = state.borrow().virtual_pointer_manager {
                let pointer = VirtualPointer::new(manager, target_output, &qhandle);
                pointer.move_to(h.x, h.y, target_info.width, target_info.height);
                pointer.click(MouseButton::Left);
            } else {
                warn!("Virtual pointer manager protocol binding missing. Cannot simulate warp.");
            }

            // Execute select callback
            Config::execute_callback(
                &config.on_select_cmd,
                h.x,
                h.y,
                target_info.width,
                target_info.height,
            )?;

            // Graceful exit callback spawner
            Config::execute_callback(
                &config.on_exit_cmd,
                0,
                0,
                target_info.width,
                target_info.height,
            )?;

            Ok((h.x, h.y, h.screen))
        } else {
            error!(
                "Label '{:?}' did not match any active hint grid entries.",
                label
            );
            Err(anyhow::anyhow!("Label mismatch"))
        }
    }

    /// Directly warping cursor to physical coordinates, optionally triggering button clicks
    pub fn move_to(
        x: i32,
        y: i32,
        click: Option<MouseButton>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let (_conn, state, qhandle, active_outputs) = Self::setup_headless()?;
        let (target_output, target_info) = active_outputs
            .first()
            .map(|(o, info)| (Some(o), info.clone()))
            .unwrap_or((
                None,
                OutputInfo {
                    name: "default".to_string(),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale: 1,
                },
            ));

        let clamped_x = x.clamp(0, target_info.width);
        let clamped_y = y.clamp(0, target_info.height);
        if clamped_x != x || clamped_y != y {
            warn!(
                "Coordinates ({}, {}) out of screen bounds ({}x{}), clamping to ({}, {})",
                x, y, target_info.width, target_info.height, clamped_x, clamped_y
            );
        }

        info!(
            "Headless move_to resolved coordinates: ({}, {}) on screen {:?}",
            clamped_x, clamped_y, target_info.name
        );

        if let Some(ref manager) = state.borrow().virtual_pointer_manager {
            let pointer = VirtualPointer::new(manager, target_output, &qhandle);
            pointer.move_to(clamped_x, clamped_y, target_info.width, target_info.height);

            if let Some(btn) = click {
                pointer.click(btn);
            }
        } else {
            warn!("Virtual pointer manager protocol binding missing. Cannot simulate warp.");
        }

        // Trigger callbacks
        Config::execute_callback(
            &config.on_select_cmd,
            clamped_x,
            clamped_y,
            target_info.width,
            target_info.height,
        )?;
        Config::execute_callback(
            &config.on_exit_cmd,
            0,
            0,
            target_info.width,
            target_info.height,
        )?;

        Ok(())
    }

    /// Directly moving cursor by relative offsets, optionally triggering button clicks
    pub fn move_by(
        dx: i32,
        dy: i32,
        click: Option<MouseButton>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let (_conn, state, qhandle, active_outputs) = Self::setup_headless()?;
        let (target_output, target_info) = active_outputs
            .first()
            .map(|(o, info)| (Some(o), info.clone()))
            .unwrap_or((
                None,
                OutputInfo {
                    name: "default".to_string(),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale: 1,
                },
            ));

        info!(
            "Headless move_by resolved offsets: ({}, {}) on screen {:?}",
            dx, dy, target_info.name
        );

        if let Some(ref manager) = state.borrow().virtual_pointer_manager {
            let pointer = VirtualPointer::new(manager, target_output, &qhandle);
            pointer.move_by(dx as f64, dy as f64);

            if let Some(btn) = click {
                pointer.click(btn);
            }
        } else {
            warn!("Virtual pointer manager protocol binding missing. Cannot simulate warp.");
        }

        // Trigger exit callback only (relative movement does not warp cursor with absolute callback)
        Config::execute_callback(
            &config.on_exit_cmd,
            0,
            0,
            target_info.width,
            target_info.height,
        )?;

        Ok(())
    }
}
