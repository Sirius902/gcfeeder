mod analog_scaling;
mod calibration;
mod center_calibration;
mod ess_inversion;
pub mod wii;

pub use analog_scaling::*;
pub use calibration::*;
pub use center_calibration::*;
pub use ess_inversion::*;
use gcinput::Input;

pub trait Layer: Send {
    fn name(&self) -> &'static str;
    fn apply(&mut self, input: Option<Input>) -> Option<Input>;
}
