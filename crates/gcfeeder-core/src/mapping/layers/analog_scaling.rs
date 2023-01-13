use gcinput::{Input, Stick, STICK_RANGE};

use crate::mapping;

pub struct AnalogScaling {
    scale: f64,
}

impl AnalogScaling {
    pub const fn new(scale: f64) -> Self {
        Self { scale }
    }

    pub fn scale_stick(&self, stick: Stick) -> Stick {
        let center = STICK_RANGE.center as f64;
        stick.map(|n| ((n as f64 - center).mul_add(self.scale, center)) as u8)
    }
}

impl mapping::Layer for AnalogScaling {
    fn name(&self) -> &'static str {
        "Scaled"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(|input| Input {
            main_stick: self.scale_stick(input.main_stick),
            c_stick: self.scale_stick(input.c_stick),
            ..input
        })
    }
}
