use log::warn;

use crate::{
    calibration::{SticksCalibration, TriggersCalibration},
    mapping,
};

pub struct Calibration {
    stick_data: Option<SticksCalibration>,
    trigger_data: Option<TriggersCalibration>,
    stick_bad: bool,
    trigger_bad: bool,
}

impl Calibration {
    pub fn new(
        stick_data: Option<SticksCalibration>,
        trigger_data: Option<TriggersCalibration>,
    ) -> Self {
        Self {
            stick_data,
            trigger_data,
            stick_bad: false,
            trigger_bad: false,
        }
    }
}

impl mapping::Layer for Calibration {
    fn name(&self) -> &'static str {
        "Calibrated"
    }

    fn apply(&mut self, mut input: Option<gcinput::Input>) -> Option<gcinput::Input> {
        if let Some(input) = input.as_mut() {
            if !self.stick_bad {
                if let Some(calibration) = self.stick_data.as_ref() {
                    if let Ok(i) = calibration.map(*input) {
                        *input = i;
                    } else {
                        self.stick_bad = true;
                        warn!("Ignoring bad stick calibration");
                    }
                }
            }

            if !self.trigger_bad {
                if let Some(calibration) = self.trigger_data.as_ref() {
                    if let Ok(i) = calibration.map(*input) {
                        *input = i;
                    } else {
                        self.trigger_bad = true;
                        warn!("Ignoring bad trigger calibration");
                    }
                }
            }
        }

        input
    }
}
