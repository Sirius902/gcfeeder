use gcinput::{Input, Rumble};

use super::Bridge;

pub struct UInputBridge;

impl Bridge for UInputBridge {
    fn driver_name(&self) -> &'static str {
        "uinput"
    }

    fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        Ok(())
    }

    fn rumble_state(&self) -> Rumble {
        Rumble::Off
    }

    fn notify_rumble_consumed(&self) {}
}
