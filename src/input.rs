#![allow(dead_code)]

pub struct InputHandler {
    // Keyboard / Seat grab state goes here
}

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        // TODO: Bind wl_seat and keyboard events
        Ok(Self {})
    }

    pub fn start_listening<F>(&mut self, mut _callback: F) -> anyhow::Result<()>
    where
        F: FnMut(String) -> bool, // String represents input, F returning true means exit
    {
        // TODO: XKB keyboard parsing loop
        Ok(())
    }
}
