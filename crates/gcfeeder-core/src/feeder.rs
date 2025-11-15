use serde::{Deserialize, Serialize};

use crate::calibration::{SticksCalibration, TriggersCalibration};
use crate::driver::DriverType;
#[cfg(target_os = "windows")]
use crate::driver::vigem::Config as ViGEmConfig;
use crate::layers::EssInversion;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Config {
    pub driver: DriverType,
    pub rumble: RumbleSetting,
    pub analog_scale: f64,
    #[cfg(target_os = "windows")]
    pub vigem_config: ViGEmConfig,
    pub calibration: CalibrationConfig,
    pub ess: EssConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            driver: Default::default(),
            rumble: Default::default(),
            analog_scale: 1.0,
            #[cfg(target_os = "windows")]
            vigem_config: Default::default(),
            calibration: Default::default(),
            ess: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub enabled: bool,
    pub stick_data: Option<SticksCalibration>,
    pub trigger_data: Option<TriggersCalibration>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EssConfig {
    pub inversion_mapping: Option<EssInversion>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum RumbleSetting {
    #[default]
    On,
    Off,
}

impl RumbleSetting {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::On, Self::Off]
    }
}
