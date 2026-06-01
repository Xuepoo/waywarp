#![allow(dead_code)]

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

pub struct VirtualPointer {
    // Virtual pointer state goes here
}

impl VirtualPointer {
    pub fn new() -> anyhow::Result<Self> {
        // TODO: Bind zwlr_virtual_pointer_v1
        Ok(Self {})
    }

    pub fn move_to(&mut self, _x: i32, _y: i32) -> anyhow::Result<()> {
        // TODO: Simulate move event
        Ok(())
    }

    pub fn click(&mut self, _button: MouseButton) -> anyhow::Result<()> {
        // TODO: Simulate click press and release events
        Ok(())
    }

    pub fn scroll(&mut self, _direction: ScrollDirection, _distance: i32) -> anyhow::Result<()> {
        // TODO: Simulate scroll events
        Ok(())
    }

    pub fn drag_to(
        &mut self,
        _start_x: i32,
        _start_y: i32,
        _end_x: i32,
        _end_y: i32,
    ) -> anyhow::Result<()> {
        // TODO: Simulate drag event
        Ok(())
    }
}
