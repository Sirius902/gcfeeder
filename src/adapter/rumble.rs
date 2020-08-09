use super::{Error, ALLOWED_TIMEOUT};
use rusb::{DeviceHandle, GlobalContext};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Rumble {
    Off,
    On,
}

impl From<Rumble> for u8 {
    fn from(state: Rumble) -> u8 {
        match state {
            Rumble::Off => 0,
            Rumble::On => 1,
        }
    }
}

pub struct Rumbler {
    pub(super) handle: Arc<DeviceHandle<GlobalContext>>,
    pub(super) endpoint_out: u8,
}

impl Rumbler {
    pub fn set_rumble(&self, states: [Rumble; 4]) -> Result<(), Error> {
        let payload = [
            0x11,
            states[0].into(),
            states[1].into(),
            states[2].into(),
            states[3].into(),
        ];

        let _bytes_written = self
            .handle
            .write_interrupt(self.endpoint_out, &payload, ALLOWED_TIMEOUT)
            .map_err(Error::Rusb)?;

        Ok(())
    }

    pub fn reset_rumble(&self) -> Result<(), Error> {
        self.set_rumble([Rumble::Off, Rumble::Off, Rumble::Off, Rumble::Off])
    }
}
