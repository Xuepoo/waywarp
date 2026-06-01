#![allow(dead_code)]

use crate::config::Config;
use crate::hint::HintGrid;

use std::os::fd::AsRawFd;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{wl_buffer, wl_compositor, wl_output, wl_registry, wl_shm, wl_shm_pool, wl_surface},
};

use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

/// Active state tracking for our Wayland connection
pub struct AppState {
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
    fn new() -> Self {
        Self {
            running: true,
            configured: false,
            compositor: None,
            shm: None,
            layer_shell: None,
            outputs: Vec::new(),
            layer_surfaces: Vec::new(),
            width: 0,
            height: 0,
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

// -----------------------------------------------------------------------------
// Renderer Implementation
// -----------------------------------------------------------------------------

pub struct Renderer {
    conn: Connection,
    state: Arc<Mutex<AppState>>,
}

impl Renderer {
    pub fn new() -> anyhow::Result<Self> {
        info!("Connecting to Wayland compositor...");
        let conn = Connection::connect_to_env().map_err(|e| {
            error!("Could not connect to Wayland env: {:?}", e);
            anyhow::anyhow!("Wayland connection failed")
        })?;

        let state = Arc::new(Mutex::new(AppState::new()));

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
        event_queue.roundtrip(&mut *self.state.lock().unwrap())?;
        event_queue.roundtrip(&mut *self.state.lock().unwrap())?; // Run twice to flush output specs

        let (compositor, shm, layer_shell, output_info) = {
            let s = self.state.lock().unwrap();
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
        event_queue.roundtrip(&mut *self.state.lock().unwrap())?;

        let (width, height) = {
            let s = self.state.lock().unwrap();
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

        // 1. Draw solid overlay layout with soft transparent background
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.2); // Very soft dark tint across background
        cr.set_operator(cairo::Operator::Source);
        cr.paint()?;

        // 2. Draw mock demo hint labels to verify Pango rendering is functional
        let mock_hints = vec![
            ("aa", width / 3, height / 2),
            ("bb", (width * 2) / 3, height / 2),
        ];

        for (label, x, y) in mock_hints {
            let label_w = 40.0;
            let label_h = 30.0;

            // Draw rounded rect background
            cr.set_operator(cairo::Operator::Over);
            cr.set_source_rgba(
                config.hint_bg[0],
                config.hint_bg[1],
                config.hint_bg[2],
                config.hint_bg[3],
            );

            // Render smooth path
            let r = config.hint_border_radius;
            cr.new_sub_path();
            cr.arc(
                x as f64 - label_w / 2.0 + r,
                y as f64 - label_h / 2.0 + r,
                r,
                180.0 * std::f64::consts::PI / 180.0,
                270.0 * std::f64::consts::PI / 180.0,
            );
            cr.arc(
                x as f64 + label_w / 2.0 - r,
                y as f64 - label_h / 2.0 + r,
                r,
                270.0 * std::f64::consts::PI / 180.0,
                360.0 * std::f64::consts::PI / 180.0,
            );
            cr.arc(
                x as f64 + label_w / 2.0 - r,
                y as f64 + label_h / 2.0 - r,
                r,
                0.0 * std::f64::consts::PI / 180.0,
                90.0 * std::f64::consts::PI / 180.0,
            );
            cr.arc(
                x as f64 - label_w / 2.0 + r,
                y as f64 + label_h / 2.0 - r,
                r,
                90.0 * std::f64::consts::PI / 180.0,
                180.0 * std::f64::consts::PI / 180.0,
            );
            cr.close_path();
            cr.fill()?;

            // Render font layout text
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

            let extents = cr.text_extents(label)?;
            cr.move_to(
                x as f64 - extents.width() / 2.0 - extents.x_bearing(),
                y as f64 - extents.height() / 2.0 - extents.y_bearing(),
            );
            cr.show_text(label)?;
        }

        cairo_surface.flush();

        // Attach populated buffer and commit surface
        wl_surface.attach(Some(&buffer), 0, 0);
        wl_surface.damage(0, 0, width, height);
        wl_surface.commit();

        info!("Renders committed successfully to the Compositor.");

        // Loop momentarily for rendering presentation (interactive testing)
        let mut loop_count = 0;
        while self.state.lock().unwrap().running && loop_count < 100 {
            event_queue.roundtrip(&mut *self.state.lock().unwrap())?;
            std::thread::sleep(std::time::Duration::from_millis(30));
            loop_count += 1;
        }

        // Clean up resources
        unsafe {
            libc::munmap(ptr as *mut libc::c_void, size);
        }

        Ok(())
    }
}
