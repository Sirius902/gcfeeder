use enum_dispatch::enum_dispatch;
use enum_iterator::Sequence;
use gcinput::{Input, Rumble};
use serde::{Deserialize, Serialize};

use crate::feeder;

#[cfg(target_os = "linux")]
pub mod evdev;
pub mod rumble;
#[cfg(windows)]
pub mod vigem;

pub type Result<T> = std::result::Result<T, Error>;

#[enum_dispatch]
pub trait Bridge {
    fn driver_name(&self) -> &'static str;
    fn feed(&self, input: &Option<Input>) -> Result<()>;
    fn rumble_state(&self) -> Rumble;
    fn notify_rumble_consumed(&self);
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(windows)]
    #[error("vigem: {0}")]
    ViGEm(#[from] vigem_client::Error),
    #[cfg(target_os = "linux")]
    #[error("evdev: {0}")]
    Evdev(#[from] evdev::Error),
}

#[enum_dispatch(Bridge)]
pub enum BridgeImpl {
    #[cfg(windows)]
    ViGEm(vigem::ViGEmBridge),
    #[cfg(target_os = "linux")]
    Evdev(evdev::EvdevBridge),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Sequence)]
#[serde(rename_all = "lowercase")]
pub enum Driver {
    #[cfg(windows)]
    ViGEm,
    #[cfg(target_os = "linux")]
    Evdev,
}

impl Driver {
    pub fn create_bridge(self, #[allow(unused)] config: &feeder::Config) -> Result<BridgeImpl> {
        match self {
            #[cfg(windows)]
            Self::ViGEm => {
                vigem::ViGEmBridge::new(config.vigem_config, vigem_client::Client::connect()?)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            #[cfg(target_os = "linux")]
            Self::Evdev => Ok(evdev::EvdevBridge::new().into()),
        }
    }
}

impl Default for Driver {
    fn default() -> Self {
        Self::first().expect("No input drivers available")
    }
}
