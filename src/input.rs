#![allow(dead_code)]

use crate::render::AppState;
use tracing::{info, warn};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_keyboard, wl_seat},
};
use xkbcommon::xkb;

/// Core interface stub to fulfill skeleton guidelines
pub struct InputHandler;

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}

// -----------------------------------------------------------------------------
// Dispatch Implementations for Seat & Keyboard Input Grabbing
// -----------------------------------------------------------------------------

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                let has_keyboard = match capabilities {
                    wayland_client::WEnum::Value(caps) => {
                        caps.contains(wl_seat::Capability::Keyboard)
                    }
                    _ => false,
                };
                if has_keyboard {
                    info!("Keyboard capability advertised by compositor seat. Binding...");
                    state.keyboard = Some(seat.get_keyboard(qhandle, ()));
                } else {
                    warn!("Seat keyboard capability removed by compositor.");
                    state.keyboard = None;
                }
            }
            wl_seat::Event::Name { name } => {
                info!("Wayland active seat name: {}", name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        state: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                if format == wayland_client::WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
                    info!(
                        "Loading compositor dynamic xkb keymap layout (size: {} B)...",
                        size
                    );
                    let ctx = state.xkb_context.as_ref().unwrap();
                    let keymap = unsafe {
                        xkb::Keymap::new_from_fd(
                            ctx,
                            fd,
                            size as usize,
                            xkb::KEYMAP_FORMAT_TEXT_V1,
                            xkb::KEYMAP_COMPILE_NO_FLAGS,
                        )
                    };

                    if let Ok(Some(km)) = keymap {
                        let xkb_state = xkb::State::new(&km);
                        state.xkb_keymap = Some(km);
                        state.xkb_state = Some(xkb_state);
                        info!("XKB Keyboard state compiled successfully.");
                    } else {
                        warn!("Failed to compile keymap layout from compositor descriptor.");
                    }
                }
            }
            wl_keyboard::Event::Key {
                serial: _,
                time: _,
                key,
                state: key_state,
            } => {
                #[allow(clippy::collapsible_if)]
                if key_state == wayland_client::WEnum::Value(wl_keyboard::KeyState::Pressed) {
                    if let Some(xkb_state) = &state.xkb_state {
                        let xkb_keycode = key + 8;
                        let keysym = xkb_state.key_get_one_sym(xkb_keycode.into());

                        if keysym == xkb::keysyms::KEY_Escape.into() {
                            info!("Escape key detected. Canceling selection...");
                            state.canceled = true;
                            state.running = false;
                        } else if keysym == xkb::keysyms::KEY_BackSpace.into() {
                            if !state.input_buf.is_empty() {
                                state.input_buf.pop();
                                info!(
                                    "Backspace key detected. Prefix buffer: {:?}",
                                    state.input_buf
                                );
                            }
                        } else if keysym == xkb::keysyms::KEY_Return.into() {
                            info!("Return/Enter key detected. Forcing select confirm.");
                            state.selection_made = true;
                            state.running = false;
                        } else {
                            // Translate keysym to character entry
                            let sym_char = xkb_state.key_get_utf8(xkb_keycode.into());
                            if !sym_char.is_empty() {
                                // Filter alphabetical lowercase entry to match typical grid configs
                                for c in sym_char.chars() {
                                    if c.is_ascii_alphabetic() {
                                        state.input_buf.push(c.to_ascii_lowercase());
                                    }
                                }
                                info!("Keystroke registered. Prefix buffer: {:?}", state.input_buf);
                            }
                        }
                    }
                }
            }
            wl_keyboard::Event::Enter {
                serial: _,
                surface: _,
                keys: _,
            } => {
                info!("Keyboard focused onto transparent Overlay window.");
            }
            wl_keyboard::Event::Leave {
                serial: _,
                surface: _,
            } => {
                info!("Keyboard focus left transparent Overlay window.");
            }
            _ => {}
        }
    }
}
