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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionMode {
    Hint,
    Normal,
}

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

    // Normal Mode (cursor drive) states
    pub mode: InteractionMode,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub shift_pressed: bool,
    pub ctrl_pressed: bool,
    pub acceleration_factor: f64,
    pub click_action: Option<crate::pointer::MouseButton>,
    pub scroll_action: Option<crate::pointer::ScrollDirection>,
    pub key_bindings: Option<NormalKeyBindings>,
}

#[derive(Debug, Clone)]
pub struct NormalKeyBindings {
    pub left: Vec<u32>,
    pub right: Vec<u32>,
    pub up: Vec<u32>,
    pub down: Vec<u32>,
    pub shift: Vec<u32>,
    pub ctrl: Vec<u32>,
    pub click_left: Vec<u32>,
    pub click_right: Vec<u32>,
    pub click_middle: Vec<u32>,
    pub scroll_up: Vec<u32>,
    pub scroll_down: Vec<u32>,
    pub exit: Vec<u32>,
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

            mode: InteractionMode::Hint,
            left_pressed: false,
            right_pressed: false,
            up_pressed: false,
            down_pressed: false,
            shift_pressed: false,
            ctrl_pressed: false,
            acceleration_factor: 1.0,
            click_action: None,
            scroll_action: None,
            key_bindings: None,
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

struct OverlayInstance {
    output: wl_output::WlOutput,
    info: OutputInfo,
    wl_surface: wl_surface::WlSurface,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    buffer: wl_buffer::WlBuffer,
    ptr: *mut u8,
    size: usize,
    cairo_surface: cairo::ImageSurface,
    cairo_context: cairo::Context,
    screen_index: u32,
    monitor_char: char,
}

pub struct Renderer {
    conn: Connection,
    pub state: Rc<RefCell<AppState>>,
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
        let bindings = NormalKeyBindings {
            left: resolve_keysyms(&config.key_left),
            right: resolve_keysyms(&config.key_right),
            up: resolve_keysyms(&config.key_up),
            down: resolve_keysyms(&config.key_down),
            shift: resolve_keysyms(&config.key_shift),
            ctrl: resolve_keysyms(&config.key_ctrl),
            click_left: resolve_keysyms(&config.key_click_left),
            click_right: resolve_keysyms(&config.key_click_right),
            click_middle: resolve_keysyms(&config.key_click_middle),
            scroll_up: resolve_keysyms(&config.key_scroll_up),
            scroll_down: resolve_keysyms(&config.key_scroll_down),
            exit: resolve_keysyms(&config.key_exit),
        };
        self.state.borrow_mut().key_bindings = Some(bindings);

        let mut event_queue = self.conn.new_event_queue();
        let qhandle = event_queue.handle();

        // Setup registry binding
        let _registry = self.conn.display().get_registry(&qhandle, ());

        info!("Retrieving active registry globals...");
        event_queue.roundtrip(&mut *self.state.borrow_mut())?;
        event_queue.roundtrip(&mut *self.state.borrow_mut())?; // Run twice to flush output specs

        // Filter and collect all fully resolved active outputs
        let active_outputs: Vec<(wl_output::WlOutput, OutputInfo)> = {
            let s = self.state.borrow();
            s.outputs
                .iter()
                .filter_map(|(o, info)| info.clone().map(|i| (o.clone(), i)))
                .filter(|(_, info)| info.width > 0 && info.height > 0)
                .collect()
        };

        let active_outputs = if active_outputs.is_empty() {
            info!("No active outputs resolved from compositor. Falling back to default.");
            let fallback_output = {
                let s = self.state.borrow();
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

        let is_multi = active_outputs.len() > 1;
        info!(
            "Detected active outputs: {}. Multi-monitor mode: {}",
            active_outputs.len(),
            is_multi
        );

        let (compositor, shm, layer_shell) = {
            let s = self.state.borrow();
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
            )
        };

        let chars = HintGrid::get_unique_chars(&config.hint_chars);
        let mut instances = Vec::new();

        for (i, (output, info)) in active_outputs.iter().enumerate() {
            let screen_index = i as u32;
            let monitor_char = chars[i % chars.len()];

            info!(
                "Spawning Layer Shell Overlay Window on monitor: {:?} ({:?}) with index {} char '{}'",
                info.name, output, screen_index, monitor_char
            );

            let wl_surface = compositor.create_surface(&qhandle, ());
            let layer_surface = layer_shell.get_layer_surface(
                &wl_surface,
                Some(output), // Bind explicitly to this monitor
                zwlr_layer_shell_v1::Layer::Overlay,
                "waywarp".to_string(),
                &qhandle,
                (),
            );

            // Anchors matching edges
            layer_surface.set_size(info.width as u32, info.height as u32);
            layer_surface.set_anchor(
                zwlr_layer_surface_v1::Anchor::Top
                    | zwlr_layer_surface_v1::Anchor::Bottom
                    | zwlr_layer_surface_v1::Anchor::Left
                    | zwlr_layer_surface_v1::Anchor::Right,
            );

            // Keyboard grab exclusively on the primary monitor overlay
            if i == 0 {
                layer_surface.set_keyboard_interactivity(
                    zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
                );
            } else {
                layer_surface
                    .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
            }
            layer_surface.set_exclusive_zone(-1); // Transparent click through allowed

            wl_surface.commit();

            // Setup SHM buffer matching monitor dimensions
            let (buffer, ptr, size) =
                Self::create_shm_buffer(&shm, info.width, info.height, &qhandle)?;
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
            let cairo_surface = cairo::ImageSurface::create_for_data(
                slice,
                cairo::Format::ARgb32,
                info.width,
                info.height,
                info.width * 4,
            )?;
            let cairo_context = cairo::Context::new(&cairo_surface)?;

            instances.push(OverlayInstance {
                output: output.clone(),
                info: info.clone(),
                wl_surface,
                layer_surface,
                buffer,
                ptr,
                size,
                cairo_surface,
                cairo_context,
                screen_index,
                monitor_char,
            });
        }

        // Registry check
        {
            let mut s = self.state.borrow_mut();
            for inst in &instances {
                s.layer_surfaces.push(inst.layer_surface.clone());
            }
        }

        // Wait for configure roundtrip
        event_queue.roundtrip(&mut *self.state.borrow_mut())?;

        // Global hint grid initialization
        let mut active_grid = if _grid.is_element_based {
            _grid.clone()
        } else if is_multi {
            let mut all_hints = Vec::new();
            for inst in &instances {
                let grid = HintGrid::generate_first_pass(
                    inst.info.width,
                    inst.info.height,
                    &config.hint_chars,
                    inst.screen_index,
                    true,
                    Some(inst.monitor_char),
                );
                all_hints.extend(grid.hints);
            }
            let mut g = HintGrid::new(0, 0, 0);
            g.hints = all_hints;
            g
        } else {
            let inst = &instances[0];
            HintGrid::generate_first_pass(
                inst.info.width,
                inst.info.height,
                &config.hint_chars,
                0,
                false,
                None,
            )
        };

        // Multi-monitor repaint closure
        let app_state_ref = self.state.clone();
        let draw_grid = move |instances: &mut [OverlayInstance],
                              grid: &HintGrid,
                              prefix: &str|
              -> anyhow::Result<()> {
            let is_normal = app_state_ref.borrow().mode == InteractionMode::Normal;
            for inst in instances {
                let cr = &inst.cairo_context;
                let info = &inst.info;

                if is_normal {
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                    cr.set_operator(cairo::Operator::Source);
                    cr.paint()?;
                } else {
                    let has_matches = grid
                        .hints
                        .iter()
                        .any(|h| h.screen == inst.screen_index && h.label.starts_with(prefix));

                    if !prefix.is_empty() && !has_matches {
                        // Repaint screen as fully transparent if no matching hints are left
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                        cr.set_operator(cairo::Operator::Source);
                        cr.paint()?;
                    } else {
                        cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
                        cr.set_operator(cairo::Operator::Source);
                        cr.paint()?;

                        for hint in &grid.hints {
                            if hint.screen != inst.screen_index {
                                continue;
                            }
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
                    }
                }
                inst.cairo_surface.flush();

                inst.wl_surface.attach(Some(&inst.buffer), 0, 0);
                inst.wl_surface.damage(0, 0, info.width, info.height);
                inst.wl_surface.commit();
            }
            Ok(())
        };

        // Draw initial frame
        draw_grid(&mut instances, &active_grid, "")?;

        info!("Initial multi-monitor frames committed. Awaiting Seat keyboard events...");

        let mut current_prefix = String::new();
        let mut refinement_level = 1;

        while self.state.borrow().running {
            event_queue.roundtrip(&mut *self.state.borrow_mut())?;

            if self.state.borrow().mode == InteractionMode::Normal {
                let (left, right, up, down, shift, ctrl, click_act, scroll_act, canceled) = {
                    let s = self.state.borrow();
                    (
                        s.left_pressed,
                        s.right_pressed,
                        s.up_pressed,
                        s.down_pressed,
                        s.shift_pressed,
                        s.ctrl_pressed,
                        s.click_action,
                        s.scroll_action,
                        s.canceled,
                    )
                };

                if canceled {
                    break;
                }

                // Handle relative movement
                let mut dx = 0.0;
                let mut dy = 0.0;
                let moving = left || right || up || down;

                if moving {
                    // Update acceleration
                    let mut s = self.state.borrow_mut();
                    let current_acc = s.acceleration_factor;
                    s.acceleration_factor = (current_acc + 0.15).min(4.0);

                    let base_speed = 10.0;
                    let speed = if shift {
                        base_speed * 3.0
                    } else if ctrl {
                        base_speed / 4.0
                    } else {
                        base_speed
                    };

                    let acc = s.acceleration_factor;
                    let x_dir = (right as i32 - left as i32) as f64;
                    let y_dir = (down as i32 - up as i32) as f64;
                    dx = x_dir * speed * acc;
                    dy = y_dir * speed * acc;
                } else {
                    self.state.borrow_mut().acceleration_factor = 1.0;
                }

                if dx != 0.0 || dy != 0.0 {
                    let manager_opt = self.state.borrow().virtual_pointer_manager.clone();
                    if let Some(manager) = manager_opt {
                        let pointer = crate::pointer::VirtualPointer::new(&manager, None, &qhandle);
                        pointer.move_by(dx, dy);
                    }
                }

                // Handle single action clicks
                if let Some(btn) = click_act {
                    if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                        let pointer = crate::pointer::VirtualPointer::new(manager, None, &qhandle);
                        pointer.click(btn);
                    }
                    self.state.borrow_mut().click_action = None;
                    if config.exit_on_select {
                        self.state.borrow_mut().running = false;
                        break;
                    }
                }

                // Handle scroll actions
                if let Some(dir) = scroll_act {
                    if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                        let pointer = crate::pointer::VirtualPointer::new(manager, None, &qhandle);
                        pointer.scroll(dir, 15);
                    }
                    self.state.borrow_mut().scroll_action = None;
                }

                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }

            let (prefix, selection_made, canceled) = {
                let s = self.state.borrow();
                (s.input_buf.clone(), s.selection_made, s.canceled)
            };

            if canceled {
                break;
            }

            if selection_made {
                let matched = active_grid
                    .hints
                    .iter()
                    .find(|h| h.label == prefix || h.label.starts_with(&prefix));
                if let Some(h) = matched {
                    info!(
                        "Selection forced confirm: label='{}' at ({}, {}) on screen {}",
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

                    if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                        let pointer =
                            crate::pointer::VirtualPointer::new(manager, target_output, &qhandle);
                        pointer.move_to(h.x, h.y, target_info.width, target_info.height);
                        pointer.click(crate::pointer::MouseButton::Left);
                    }
                    let _ = Config::execute_callback(
                        &config.on_select_cmd,
                        h.x,
                        h.y,
                        target_info.width,
                        target_info.height,
                    );
                }
                break;
            }

            if prefix != current_prefix {
                current_prefix = prefix.clone();
                info!("Input buffer mutated: {:?}", current_prefix);

                let matches: Vec<&crate::hint::Hint> = active_grid
                    .hints
                    .iter()
                    .filter(|h| h.label.starts_with(&current_prefix))
                    .collect();

                if matches.is_empty() {
                    self.state.borrow_mut().input_buf.clear();
                    current_prefix.clear();
                    draw_grid(&mut instances, &active_grid, "")?;
                } else if matches.len() == 1 {
                    let matched_hint = matches[0].clone();

                    if !active_grid.is_element_based && refinement_level < config.refinement_passes
                    {
                        info!(
                            "Triggering refinement pass {} for label='{}' on screen {}",
                            refinement_level + 1,
                            matched_hint.label,
                            matched_hint.screen
                        );
                        refinement_level += 1;

                        self.state.borrow_mut().input_buf.clear();
                        current_prefix.clear();

                        let target_info = active_outputs
                            .iter()
                            .enumerate()
                            .find(|(idx, _)| *idx as u32 == matched_hint.screen)
                            .map(|(_, (_, info))| info.clone())
                            .unwrap_or(OutputInfo {
                                name: "default".to_string(),
                                x: 0,
                                y: 0,
                                width: 1920,
                                height: 1080,
                                scale: 1,
                            });

                        let unique_len =
                            HintGrid::get_unique_chars(&config.hint_chars).len() as i32;
                        let cell_w = target_info.width / unique_len;
                        let cell_h = target_info.height / unique_len;
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
                            matched_hint.screen,
                        );
                        draw_grid(&mut instances, &active_grid, "")?;
                    } else {
                        info!(
                            "Warp target successfully resolved: ({}, {}) on screen {}",
                            matched_hint.x, matched_hint.y, matched_hint.screen
                        );

                        let (target_output, target_info) = active_outputs
                            .iter()
                            .enumerate()
                            .find(|(idx, _)| *idx as u32 == matched_hint.screen)
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

                        if let Some(ref manager) = self.state.borrow().virtual_pointer_manager {
                            let pointer = crate::pointer::VirtualPointer::new(
                                manager,
                                target_output,
                                &qhandle,
                            );
                            pointer.move_to(
                                matched_hint.x,
                                matched_hint.y,
                                target_info.width,
                                target_info.height,
                            );
                            pointer.click(crate::pointer::MouseButton::Left);
                        }
                        let _ = Config::execute_callback(
                            &config.on_select_cmd,
                            matched_hint.x,
                            matched_hint.y,
                            target_info.width,
                            target_info.height,
                        );
                        self.state.borrow_mut().running = false;
                    }
                } else {
                    draw_grid(&mut instances, &active_grid, &current_prefix)?;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // Final exit callback
        if let Some(inst) = instances.first() {
            let _ = Config::execute_callback(
                &config.on_exit_cmd,
                0,
                0,
                inst.info.width,
                inst.info.height,
            );
        } else {
            let _ = Config::execute_callback(&config.on_exit_cmd, 0, 0, 1920, 1080);
        }

        // Clean up resources securely
        for inst in instances {
            unsafe {
                libc::munmap(inst.ptr as *mut libc::c_void, inst.size);
            }
        }

        Ok(())
    }
}

fn resolve_keysyms(comma_separated: &str) -> Vec<u32> {
    comma_separated
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|name| {
            let sym = xkbcommon::xkb::keysym_from_name(name, xkbcommon::xkb::KEYSYM_NO_FLAGS);
            if sym == xkbcommon::xkb::keysyms::KEY_NoSymbol.into() {
                None
            } else {
                Some(sym.raw())
            }
        })
        .collect()
}
