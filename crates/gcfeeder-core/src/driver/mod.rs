use async_trait::async_trait;
use gcinput::Input;
use serde::{Deserialize, Serialize};

use crate::adapter::Port;
use crate::feeder;

#[cfg(target_os = "linux")]
pub mod evdev;
pub mod rumble;
#[cfg(target_os = "windows")]
pub mod vigem;

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait Driver: Send + Sync {
    fn name(&self) -> &'static str;
    async fn feed(&self, input: &Option<Input>) -> Result<()>;
    async fn recv_rumble_strength(&self) -> Result<u8>;
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
pub enum DriverType {
    #[cfg(target_os = "windows")]
    ViGEm,
    #[cfg(target_os = "linux")]
    Evdev,
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    None,
}

impl DriverType {
    pub fn create(
        self,
        #[allow(unused)] port: Port,
        #[allow(unused)] config: &feeder::Config,
    ) -> Result<Option<Box<dyn Driver>>> {
        match self {
            #[cfg(target_os = "windows")]
            Self::ViGEm => Ok(Some(Box::new(vigem::Driver::new(
                config.vigem_config,
                vigem_client::Client::connect()?,
            )?))),
            #[cfg(target_os = "linux")]
            Self::Evdev => Ok(Some(Box::new(evdev::Driver::new(port)))),
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            Self::None => Ok(None),
        }
    }
}

impl Default for DriverType {
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
            Self::None
        }
    }
}
