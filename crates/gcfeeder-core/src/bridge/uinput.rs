use std::{fs, io};

use gcinput::{Input, Rumble};
use input_linux::{sys, InputId, UInputHandle};

use super::Bridge;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO: Only create device when adapter port is plugged
pub struct UInputBridge {
    device: UInputHandle<fs::File>,
}

impl UInputBridge {
    pub fn new() -> Result<Self> {
        let device = UInputHandle::new(fs::File::create("/dev/uinput")?);

        // Use Xbox360 controller vid and pid for now
        let input_id = InputId {
            bustype: sys::BUS_VIRTUAL,
            vendor: 0x045E,
            product: 0x028E,
            version: 1,
        };

        device.set_evbit(input_linux::EventKind::Absolute)?;
        device.set_evbit(input_linux::EventKind::Key)?;
        device.set_keybit(input_linux::Key::ButtonTrigger)?;

        device.set_keybit(input_linux::Key::Button0)?;
        device.set_keybit(input_linux::Key::Button1)?;
        device.set_keybit(input_linux::Key::Button2)?;

        device.create(&input_id, b"gamecube-controller", 1, &[])?;

        Ok(Self { device })
    }
}

impl Bridge for UInputBridge {
    fn driver_name(&self) -> &'static str {
        "uinput"
    }

    fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        let state = if input.map(|i| i.button_a).unwrap_or(false) {
            input_linux::KeyState::PRESSED
        } else {
            input_linux::KeyState::RELEASED
        };

        let _ = self
            .device
            .write(&[*input_linux::KeyEvent::new(
                input_linux::EventTime::new(0, 0),
                input_linux::Key::Button0,
                state,
            )
            .as_ref()])
            .map_err(Error::Io)?;

        Ok(())
    }

    fn rumble_state(&self) -> Rumble {
        Rumble::Off
    }

    fn notify_rumble_consumed(&self) {}
}
