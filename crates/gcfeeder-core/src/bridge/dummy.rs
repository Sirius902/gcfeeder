use gcinput::{Input, Rumble};

use super::{Bridge, Result};

pub struct DummyBridge;

impl Bridge for DummyBridge {
    fn driver_name(&self) -> &'static str {
        "dummy"
    }

    fn feed(&self, _input: &Option<Input>) -> Result<()> {
        Ok(())
    }

    fn rumble_state(&self) -> Rumble {
        Rumble::Off
    }

    fn notify_rumble_consumed(&self) {}
}
