use gcinput::{Input, Rumble};
use serde::{Deserialize, Serialize};

use crate::feeder;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub mod dummy;
#[cfg(target_os = "linux")]
pub mod evdev;
pub mod rumble;
#[cfg(target_os = "windows")]
pub mod vigem;

// FUTURE(Sirius902) Rework bridges to be async?

pub type Result<T> = std::result::Result<T, Error>;

pub trait Bridge {
    fn driver_name(&self) -> &'static str;
    fn feed(&self, input: &Option<Input>) -> Result<()>;
    fn rumble_state(&self) -> Rumble;
    fn notify_rumble_consumed(&self);
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(target_os = "windows")]
    #[error("vigem: {0}")]
    ViGEm(#[from] vigem_client::Error),
    #[cfg(target_os = "linux")]
    #[error("evdev: {0}")]
    Evdev(#[from] evdev::Error),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Driver {
    #[cfg(target_os = "windows")]
    ViGEm,
    #[cfg(target_os = "linux")]
    Evdev,
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    Dummy,
}

impl Driver {
    pub fn create_bridge(
        self,
        #[allow(unused)] config: &feeder::Config,
    ) -> Result<Box<dyn Bridge>> {
        match self {
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            Self::Dummy => Ok(Box::new(dummy::DummyBridge)),
            #[cfg(target_os = "windows")]
            Self::ViGEm => {
                vigem::ViGEmBridge::new(config.vigem_config, vigem_client::Client::connect()?)
                    .map(Into::into)
                    .map_err(Into::into)
            }
            #[cfg(target_os = "linux")]
            Self::Evdev => Ok(Box::new(evdev::EvdevBridge::new())),
        }
    }
}

impl Default for Driver {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::ViGEm
        }
        #[cfg(target_os = "linux")]
        {
            Self::Evdev
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Self::Dummy
        }
    }
}
