use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{STICK_RANGE, TRIGGER_RANGE};
use nalgebra::Vector2;

use crate::mapping;

#[derive(Default)]
pub struct CenterCalibration {
    center_data: Option<DriftData>,
}

struct DriftData {
    pub main_stick: Vector2<i16>,
    pub c_stick: Vector2<i16>,

    pub trigger_left: i16,
    pub trigger_right: i16,
}

impl DriftData {
    pub fn new(input: gcinput::Input) -> Self {
        Self {
            main_stick: input
                .main_stick
                .to_vector()
                .map(|n| i16::from(STICK_RANGE.center) - i16::from(n)),
            c_stick: input
                .c_stick
                .to_vector()
                .map(|n| i16::from(STICK_RANGE.center) - i16::from(n)),

            trigger_left: i16::from(TRIGGER_RANGE.min) - i16::from(input.left_trigger),
            trigger_right: i16::from(TRIGGER_RANGE.min) - i16::from(input.right_trigger),
        }
    }
}

impl mapping::Layer for CenterCalibration {
    fn name(&self) -> &'static str {
        "Centered"
    }

    fn apply(&mut self, mut input: Option<gcinput::Input>) -> Option<gcinput::Input> {
        if let Some(input) = input.as_mut() {
            let data = self
                .center_data
                .get_or_insert_with(|| DriftData::new(*input));

            let apply_stick = |stick: &mut gcinput::Stick, drift: &Vector2<i16>| {
                let corrected = {
                    let mut c = stick.to_vector().map(i16::from);
                    c.zip_apply(drift, |axis, drift| *axis += drift);
                    c
                };

                *stick = corrected
                    .map(|n| n.approx_as::<u8>().unwrap_or_saturate())
                    .into();
            };

            let apply_trigger = |trigger: &mut u8, drift: i16| {
                *trigger = (i16::from(*trigger) + drift)
                    .approx_as::<u8>()
                    .unwrap_or_saturate();
            };

            apply_stick(&mut input.main_stick, &data.main_stick);
            apply_stick(&mut input.c_stick, &data.c_stick);

            apply_trigger(&mut input.left_trigger, data.trigger_left);
            apply_trigger(&mut input.right_trigger, data.trigger_right);
        } else {
            self.center_data = None;
        }

        input
    }
}
