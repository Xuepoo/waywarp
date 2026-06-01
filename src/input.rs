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
                let is_pressed =
                    key_state == wayland_client::WEnum::Value(wl_keyboard::KeyState::Pressed);
                if let Some(xkb_state) = &state.xkb_state {
                    let xkb_keycode = key + 8;
                    let keysym = xkb_state.key_get_one_sym(xkb_keycode.into());

                    if state.mode == crate::render::InteractionMode::Normal {
                        let keysym_u32 = keysym.raw();

                        let (
                            is_left,
                            is_right,
                            is_up,
                            is_down,
                            is_shift,
                            is_ctrl,
                            is_exit,
                            is_click_left,
                            is_click_right,
                            is_click_middle,
                            is_scroll_up,
                            is_scroll_down,
                        ) = if let Some(ref bindings) = state.key_bindings {
                            (
                                bindings.left.contains(&keysym_u32),
                                bindings.right.contains(&keysym_u32),
                                bindings.up.contains(&keysym_u32),
                                bindings.down.contains(&keysym_u32),
                                bindings.shift.contains(&keysym_u32),
                                bindings.ctrl.contains(&keysym_u32),
                                bindings.exit.contains(&keysym_u32),
                                bindings.click_left.contains(&keysym_u32),
                                bindings.click_right.contains(&keysym_u32),
                                bindings.click_middle.contains(&keysym_u32),
                                bindings.scroll_up.contains(&keysym_u32),
                                bindings.scroll_down.contains(&keysym_u32),
                            )
                        } else {
                            (
                                keysym == xkb::keysyms::KEY_h.into()
                                    || keysym == xkb::keysyms::KEY_H.into()
                                    || keysym == xkb::keysyms::KEY_Left.into(),
                                keysym == xkb::keysyms::KEY_l.into()
                                    || keysym == xkb::keysyms::KEY_L.into()
                                    || keysym == xkb::keysyms::KEY_Right.into(),
                                keysym == xkb::keysyms::KEY_k.into()
                                    || keysym == xkb::keysyms::KEY_K.into()
                                    || keysym == xkb::keysyms::KEY_Up.into(),
                                keysym == xkb::keysyms::KEY_j.into()
                                    || keysym == xkb::keysyms::KEY_J.into()
                                    || keysym == xkb::keysyms::KEY_Down.into(),
                                keysym == xkb::keysyms::KEY_Shift_L.into()
                                    || keysym == xkb::keysyms::KEY_Shift_R.into(),
                                keysym == xkb::keysyms::KEY_Control_L.into()
                                    || keysym == xkb::keysyms::KEY_Control_R.into(),
                                keysym == xkb::keysyms::KEY_Escape.into()
                                    || keysym == xkb::keysyms::KEY_q.into()
                                    || keysym == xkb::keysyms::KEY_Q.into(),
                                keysym == xkb::keysyms::KEY_f.into()
                                    || keysym == xkb::keysyms::KEY_F.into()
                                    || keysym == xkb::keysyms::KEY_Return.into(),
                                keysym == xkb::keysyms::KEY_d.into()
                                    || keysym == xkb::keysyms::KEY_D.into(),
                                keysym == xkb::keysyms::KEY_s.into()
                                    || keysym == xkb::keysyms::KEY_S.into(),
                                keysym == xkb::keysyms::KEY_u.into()
                                    || keysym == xkb::keysyms::KEY_U.into(),
                                keysym == xkb::keysyms::KEY_e.into()
                                    || keysym == xkb::keysyms::KEY_E.into(),
                            )
                        };

                        if is_left {
                            state.left_pressed = is_pressed;
                        } else if is_right {
                            state.right_pressed = is_pressed;
                        } else if is_up {
                            state.up_pressed = is_pressed;
                        } else if is_down {
                            state.down_pressed = is_pressed;
                        } else if is_shift {
                            state.shift_pressed = is_pressed;
                        } else if is_ctrl {
                            state.ctrl_pressed = is_pressed;
                        } else if is_pressed {
                            if is_exit {
                                info!("Exit request received in Normal Mode.");
                                state.canceled = true;
                                state.running = false;
                            } else if is_click_left {
                                info!("Left Click request in Normal Mode.");
                                state.click_action = Some(crate::pointer::MouseButton::Left);
                            } else if is_click_right {
                                info!("Right Click request in Normal Mode.");
                                state.click_action = Some(crate::pointer::MouseButton::Right);
                            } else if is_click_middle {
                                info!("Middle Click request in Normal Mode.");
                                state.click_action = Some(crate::pointer::MouseButton::Middle);
                            } else if is_scroll_up {
                                info!("Scroll Up request in Normal Mode.");
                                state.scroll_action = Some(crate::pointer::ScrollDirection::Up);
                            } else if is_scroll_down {
                                info!("Scroll Down request in Normal Mode.");
                                state.scroll_action = Some(crate::pointer::ScrollDirection::Down);
                            }
                        }
                    } else {
                        #[allow(clippy::collapsible_if)]
                        if is_pressed {
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
                                    info!(
                                        "Keystroke registered. Prefix buffer: {:?}",
                                        state.input_buf
                                    );
                                }
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
