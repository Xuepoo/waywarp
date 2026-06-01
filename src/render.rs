#![allow(dead_code)]

use crate::config::Config;
use crate::hint::HintGrid;

use std::cell::RefCell;
use std::os::fd::AsRawFd;
use std::rc::Rc;
use tracing::{error, info};

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{
        wl_buffer, wl_compositor, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
};

use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

/// Active state tracking for our Wayland connection
pub struct AppState {
    pub virtual_pointer_manager:
        Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
    pub running: bool,
    pub configured: bool,
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub shm: Option<wl_shm::WlShm>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub outputs: Vec<(wl_output::WlOutput, Option<OutputInfo>)>,
    pub layer_surfaces: Vec<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,

    // Geometry
    pub width: i32,
    pub height: i32,

    // Keyboard input state
    pub seat: Option<wayland_client::protocol::wl_seat::WlSeat>,
    pub keyboard: Option<wayland_client::protocol::wl_keyboard::WlKeyboard>,
    pub xkb_context: Option<xkbcommon::xkb::Context>,
    pub xkb_keymap: Option<xkbcommon::xkb::Keymap>,
    pub xkb_state: Option<xkbcommon::xkb::State>,
    pub input_buf: String,
    pub selection_made: bool,
    pub canceled: bool,
}

#[derive(Clone, Debug)]
pub struct OutputInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: i32,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            virtual_pointer_manager: None,
            running: true,
            configured: false,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            layer_surfaces: Vec::new(),
            width: 0,
            height: 0,
            seat: None,
            keyboard: None,
            xkb_context: Some(xkbcommon::xkb::Context::new(
                xkbcommon::xkb::CONTEXT_NO_FLAGS,
            )),
            xkb_keymap: None,
            xkb_state: None,
            input_buf: String::new(),
            selection_made: false,
            canceled: false,
        }
    }
}

// -----------------------------------------------------------------------------
// Dispatch Implementations for Wayland-client
// -----------------------------------------------------------------------------

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version: _,
        } = event
        {
            match &interface[..] {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name,
                        4,
                        qhandle,
                        (),
                    ));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(name, 1, qhandle, ()));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(
                        registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                            name,
                            1,
                            qhandle,
                            (),
                        ),
                    );
                }
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(name, 4, qhandle, ());
                    state.outputs.push((output, None));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(name, 4, qhandle, ()));
                }
                "zwlr_virtual_pointer_manager_v1" => {
                    state.virtual_pointer_manager = Some(registry.bind::<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, _, _>(name, 2, qhandle, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for AppState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Process geometry and scale factors if compositor provides them
        if let Some(idx) = state.outputs.iter().position(|(o, _)| o == output) {
            match event {
                wl_output::Event::Geometry {
                    x,
                    y,
                    physical_width: _,
                    physical_height: _,
                    subpixel: _,
                    make: _,
                    model: _,
                    transform: _,
                } => {
                    if let Some(info) = &mut state.outputs[idx].1 {
                        info.x = x;
                        info.y = y;
                    } else {
                        state.outputs[idx].1 = Some(OutputInfo {
                            name: String::new(),
                            x,
                            y,
                            width: 0,
                            height: 0,
                            scale: 1,
                        });
                    }
                }
                wl_output::Event::Mode {
                    flags: _,
                    width,
                    height,
                    refresh: _,
                } => {
                    if let Some(info) = &mut state.outputs[idx].1 {
                        info.width = width;
                        info.height = height;
                    } else {
                        state.outputs[idx].1 = Some(OutputInfo {
                            name: String::new(),
                            x: 0,
                            y: 0,
                            width,
                            height,
                            scale: 1,
                        });
                    }
                }
                wl_output::Event::Scale { factor } => {
                    if let Some(info) = &mut state.outputs[idx].1 {
                        info.scale = factor;
                    }
                }
                wl_output::Event::Name { name } => {
                    if let Some(info) = &mut state.outputs[idx].1 {
                        info.name = name;
                    }
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                surface.ack_configure(serial);
                state.configured = true;
                state.width = width as i32;
                state.height = height as i32;
                info!(
                    "Layer surface configured: width={}, height={}",
                    width, height
                );
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
                info!("Layer surface closed by compositor");
            }
            _ => {}
        }
    }
}

// Boilerplate no-ops for interfaces that don't emit events we care about
delegate_noop!(AppState: ignore wl_compositor::WlCompositor);
delegate_noop!(AppState: ignore wl_shm::WlShm);
delegate_noop!(AppState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(AppState: ignore wl_buffer::WlBuffer);
delegate_noop!(AppState: ignore wl_surface::WlSurface);
delegate_noop!(AppState: ignore zwlr_layer_shell_v1::ZwlrLayerShellV1);
delegate_noop!(AppState: ignore zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1);
delegate_noop!(AppState: ignore zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1);

// -----------------------------------------------------------------------------
// Renderer Implementation
// -----------------------------------------------------------------------------

pub struct Renderer {
    conn: Connection,
    state: Rc<RefCell<AppState>>,
}

impl Renderer {
    pub fn new() -> anyhow::Result<Self> {
        info!("Connecting to Wayland compositor...");
        let conn = Connection::connect_to_env().map_err(|e| {
            error!("Could not connect to Wayland env: {:?}", e);
            anyhow::anyhow!("Wayland connection failed")
        })?;

        let state = Rc::new(RefCell::new(AppState::new()));

        Ok(Self { conn, state })
    }

    /// Allocates shared memory backing for Cairo graphics and binds to WlBuffer
    fn create_shm_buffer(
        shm: &wl_shm::WlShm,
        width: i32,
        height: i32,
        qhandle: &QueueHandle<AppState>,
    ) -> anyhow::Result<(wl_buffer::WlBuffer, *mut u8, usize)> {
        let stride = width * 4;
        let size = (stride * height) as usize;

        // Secure a path in /dev/shm and unlink immediately
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = format!("/dev/shm/waywarp-shm-{}", timestamp);
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        let _ = std::fs::remove_file(&path); // Unlink immediately so standard cleanup occurs
        file.set_len(size as u64)?;

        // Map buffer into memory
        let fd = file.as_raw_fd();
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(anyhow::anyhow!("mmap shared memory buffer failed"));
        }

        // Wrap fd in a pool
        use std::os::fd::AsFd;
        let pool = shm.create_pool(file.as_fd(), size as i32, qhandle, ());
        let buffer = pool.create_buffer(
            0,
            width,
            height,
            stride,
            wl_shm::Format::Argb8888,
            qhandle,
            (),
        );

        Ok((buffer, ptr as *mut u8, size))
    }

    /// Primary execution loop to present interactive transparent layout overlay
    pub fn draw_overlay(&mut self, _grid: &HintGrid, config: &Config) -> anyhow::Result<()> {
        let mut event_queue = self.conn.new_event_queue();
        let qhandle = event_queue.handle();

        // Setup registry binding
        let _registry = self.conn.display().get_registry(&qhandle, ());

        info!("Retrieving active registry globals...");
        event_queue.roundtrip(&mut *self.state.borrow_mut())?;
        event_queue.roundtrip(&mut *self.state.borrow_mut())?; // Run twice to flush output specs

        let (compositor, shm, layer_shell, output_info) = {
            let s = self.state.borrow();
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
            (
                s.compositor
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("wl_compositor binding missing"))?,
                s.shm
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("wl_shm binding missing"))?,
                s.layer_shell
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("zwlr_layer_shell_v1 missing"))?,
                out,
            )
        };

        info!(
            "Spawning Layer Shell Overlay Window on monitor: {:?}",
            output_info.name
        );

        let wl_surface = compositor.create_surface(&qhandle, ());

        // Grab layer surface
        let layer_surface = layer_shell.get_layer_surface(
            &wl_surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "waywarp".to_string(),
            &qhandle,
            (),
        );

        // Standard setup: fullscreen, anchor to all edges
        layer_surface.set_size(output_info.width as u32, output_info.height as u32);
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Bottom
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
        layer_surface.set_exclusive_zone(-1); // Transparent click through allowed

        wl_surface.commit();

        // Roundtrip to wait for configure event to establish bounds
        event_queue.roundtrip(&mut *self.state.borrow_mut())?;

        let (width, height) = {
            let s = self.state.borrow();
            (s.width, s.height)
        };

        if width <= 0 || height <= 0 {
            return Err(anyhow::anyhow!("Invalid Layer surface configured bounds"));
        }

        info!(
            "Allocating {}x{} pixel SHM buffer context...",
            width, height
        );
        let (buffer, ptr, size) = Self::create_shm_buffer(&shm, width, height, &qhandle)?;

        // Connect Cairo rendering canvas directly onto shared memory region
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
        let cairo_surface = cairo::ImageSurface::create_for_data(
            slice,
            cairo::Format::ARgb32,
            width,
            height,
            width * 4,
        )?;

        let cr = cairo::Context::new(&cairo_surface)?;

        // Helper function to repaint background and filtered labels
        let draw_grid = |grid: &HintGrid, prefix: &str| -> anyhow::Result<()> {
            // 1. Solid overlay layout with soft transparent background
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
            cr.set_operator(cairo::Operator::Source);
            cr.paint()?;

            // 2. Render filtered labels
            for hint in &grid.hints {
                if !hint.label.starts_with(prefix) {
                    continue;
                }

                let label_w = hint.width as f64;
                let label_h = hint.height as f64;
                let x = hint.x as f64;
                let y = hint.y as f64;

                cr.set_operator(cairo::Operator::Over);
                cr.set_source_rgba(
                    config.hint_bg[0],
                    config.hint_bg[1],
                    config.hint_bg[2],
                    config.hint_bg[3],
                );

                let r = config.hint_border_radius;
                cr.new_sub_path();
                cr.arc(
                    x - label_w / 2.0 + r,
                    y - label_h / 2.0 + r,
                    r,
                    180.0 * std::f64::consts::PI / 180.0,
                    270.0 * std::f64::consts::PI / 180.0,
                );
                cr.arc(
                    x + label_w / 2.0 - r,
                    y - label_h / 2.0 + r,
                    r,
                    270.0 * std::f64::consts::PI / 180.0,
                    360.0 * std::f64::consts::PI / 180.0,
                );
                cr.arc(
                    x + label_w / 2.0 - r,
                    y + label_h / 2.0 - r,
                    r,
                    0.0 * std::f64::consts::PI / 180.0,
                    90.0 * std::f64::consts::PI / 180.0,
                );
                cr.arc(
                    x - label_w / 2.0 + r,
                    y + label_h / 2.0 - r,
                    r,
                    90.0 * std::f64::consts::PI / 180.0,
                    180.0 * std::f64::consts::PI / 180.0,
                );
                cr.close_path();
                cr.fill()?;

                cr.set_source_rgba(
                    config.hint_fg[0],
                    config.hint_fg[1],
                    config.hint_fg[2],
                    config.hint_fg[3],
                );

                cr.select_font_face(
                    &config.hint_font,
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Normal,
                );
                cr.set_font_size(config.hint_size as f64);

                let extents = cr.text_extents(&hint.label)?;
                cr.move_to(
                    x - extents.width() / 2.0 - extents.x_bearing(),
                    y - extents.height() / 2.0 - extents.y_bearing(),
                );
                cr.show_text(&hint.label)?;
            }

            cairo_surface.flush();
            Ok(())
        };

        // Initialize active HintGrid
        let mut active_grid =
            HintGrid::generate_first_pass(width, height, &config.hint_chars, 0, false, None);

        // Perform initial draw
        draw_grid(&active_grid, "")?;

        // Attach populated buffer and commit surface
        wl_surface.attach(Some(&buffer), 0, 0);
        wl_surface.damage(0, 0, width, height);
        wl_surface.commit();

        info!("Initial frame committed. Awaiting Wayland seat keyboard events...");

        let mut current_prefix = String::new();
        let mut refinement_level = 1;

        // Loop dynamically processing inputs
        while self.state.borrow().running {
            event_queue.roundtrip(&mut *self.state.borrow_mut())?;

            let (prefix, selection_made, canceled) = {
                let s = self.state.borrow();
                (s.input_buf.clone(), s.selection_made, s.canceled)
            };

            if canceled {
                break;
            }

            // Forced select confirm via Enter/Return
            if selection_made {
                let matched = active_grid
                    .hints
                    .iter()
                    .find(|h| h.label == prefix || h.label.starts_with(&prefix));
                if let Some(h) = matched {
                    info!(
                        "Selection forced confirm: label='{}' at ({}, {})",
                        h.label, h.x, h.y
                    );
                    if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                        let pointer = crate::pointer::VirtualPointer::new(manager, &qhandle);
                        pointer.move_to(h.x, h.y, width, height);
                        pointer.click(crate::pointer::MouseButton::Left);
                    }
                    let _ =
                        Config::execute_callback(&config.on_select_cmd, h.x, h.y, width, height);
                }
                break;
            }

            // Redraw filtered layout upon prefix buffer mutation
            if prefix != current_prefix {
                current_prefix = prefix.clone();
                info!("Input buffer mutated: {:?}", current_prefix);

                let matches: Vec<&crate::hint::Hint> = active_grid
                    .hints
                    .iter()
                    .filter(|h| h.label.starts_with(&current_prefix))
                    .collect();

                if matches.is_empty() {
                    // Reset input prefix on mismatch
                    self.state.borrow_mut().input_buf.clear();
                    current_prefix.clear();
                    draw_grid(&active_grid, "")?;
                } else if matches.len() == 1 {
                    // Match resolved!
                    let matched_hint = matches[0].clone();

                    if refinement_level < config.refinement_passes {
                        // Subdivision refinement zoom
                        info!(
                            "Triggering refinement pass {} for label='{}'",
                            refinement_level + 1,
                            matched_hint.label
                        );
                        refinement_level += 1;

                        self.state.borrow_mut().input_buf.clear();
                        current_prefix.clear();

                        // Derive subdivision boundaries
                        let unique_len =
                            HintGrid::get_unique_chars(&config.hint_chars).len() as i32;
                        let cell_w = width / unique_len;
                        let cell_h = height / unique_len;
                        let x_min = matched_hint.x - cell_w / 2;
                        let y_min = matched_hint.y - cell_h / 2;
                        let x_max = matched_hint.x + cell_w / 2;
                        let y_max = matched_hint.y + cell_h / 2;

                        active_grid = HintGrid::generate_refinement(
                            x_min,
                            y_min,
                            x_max,
                            y_max,
                            &config.hint_chars,
                            0,
                        );
                        draw_grid(&active_grid, "")?;
                    } else {
                        // Select resolved
                        info!(
                            "Warp target successfully resolved: ({}, {})",
                            matched_hint.x, matched_hint.y
                        );
                        if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                            let pointer = crate::pointer::VirtualPointer::new(manager, &qhandle);
                            pointer.move_to(matched_hint.x, matched_hint.y, width, height);
                            pointer.click(crate::pointer::MouseButton::Left);
                        }
                        let _ = Config::execute_callback(
                            &config.on_select_cmd,
                            matched_hint.x,
                            matched_hint.y,
                            width,
                            height,
                        );
                        self.state.borrow_mut().running = false;
                    }
                } else {
                    // Dynamic filtering
                    draw_grid(&active_grid, &current_prefix)?;
                }

                wl_surface.attach(Some(&buffer), 0, 0);
                wl_surface.damage(0, 0, width, height);
                wl_surface.commit();
            }

            std::thread::sleep(std::time::Duration::from_millis(16)); // Throttle loops to roughly ~60hz
        }

        let _ = Config::execute_callback(&config.on_exit_cmd, 0, 0, width, height);

        // Clean up resources
        unsafe {
            libc::munmap(ptr as *mut libc::c_void, size);
        }

        Ok(())
    }
}
