#![allow(dead_code)]

use crate::render::AppState;
use wayland_client::{
    QueueHandle,
    protocol::{wl_output, wl_pointer},
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

pub struct VirtualPointer {
    pointer: zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
}

impl VirtualPointer {
    pub fn new(
        manager: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        output: Option<&wl_output::WlOutput>,
        qhandle: &QueueHandle<AppState>,
    ) -> Self {
        let pointer = if output.is_some() {
            manager.create_virtual_pointer_with_output(None, output, qhandle, ())
        } else {
            manager.create_virtual_pointer(None, qhandle, ())
        };
        Self { pointer }
    }

    /// Warp physical cursor to absolute coordinates
    pub fn move_to(&self, x: i32, y: i32, screen_w: i32, screen_h: i32) {
        let time = 0;
        self.pointer
            .motion_absolute(time, x as u32, y as u32, screen_w as u32, screen_h as u32);
        self.pointer.frame();
    }

    /// Move physical cursor by relative coordinate offsets (dx, dy)
    pub fn move_by(&self, dx: f64, dy: f64) {
        let time = 0;
        self.pointer.motion(time, dx, dy);
        self.pointer.frame();
    }

    /// Simulate physical hardware click button events
    pub fn click(&self, button: MouseButton) {
        let btn_code = match button {
            MouseButton::Left => 0x110,   // BTN_LEFT
            MouseButton::Right => 0x111,  // BTN_RIGHT
            MouseButton::Middle => 0x112, // BTN_MIDDLE
        };
        let time = 0;

        // Press
        self.pointer
            .button(time, btn_code, wl_pointer::ButtonState::Pressed);
        self.pointer.frame();

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Release
        self.pointer
            .button(time, btn_code, wl_pointer::ButtonState::Released);
        self.pointer.frame();
    }

    /// Simulate scrolling wheel events
    pub fn scroll(&self, direction: ScrollDirection, distance: i32) {
        let time = 0;
        // Axis 0 is Vertical, Axis 1 is Horizontal in Wayland
        let axis = match direction {
            ScrollDirection::Up | ScrollDirection::Down => wl_pointer::Axis::VerticalScroll,
            ScrollDirection::Left | ScrollDirection::Right => wl_pointer::Axis::HorizontalScroll,
        };
        let value = match direction {
            ScrollDirection::Up | ScrollDirection::Left => -distance,
            ScrollDirection::Down | ScrollDirection::Right => distance,
        };

        self.pointer.axis(time, axis, value as f64);
        self.pointer.frame();
    }

    /// Emulate hardware cursor drag gesture from start to end coordinates
    pub fn drag_to(
        &self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        screen_w: i32,
        screen_h: i32,
    ) {
        self.move_to(start_x, start_y, screen_w, screen_h);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Press Left Button
        self.pointer
            .button(0, 0x110, wl_pointer::ButtonState::Pressed);
        self.pointer.frame();
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Move to end position
        self.move_to(end_x, end_y, screen_w, screen_h);
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Release
        self.pointer
            .button(0, 0x110, wl_pointer::ButtonState::Released);
        self.pointer.frame();
    }
}
