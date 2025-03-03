use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{Input, STICK_RANGE};

use crate::mapping;

#[derive(Default)]
pub struct Vc;

impl Vc {
    pub const fn new() -> Self {
        Self
    }

    pub fn apply(mut input: Input) -> Input {
        let mut main_x = i32::from(input.main_stick.x) - i32::from(STICK_RANGE.center);
        let mut main_y = i32::from(input.main_stick.y) - i32::from(STICK_RANGE.center);

        let mut main_x_f = main_x as f64 / 56.0;
        let mut main_y_f = main_y as f64 / 56.0;

        if main_x_f >= 0.0 {
            main_x_f = (1.0 - main_x_f).sqrt();
            main_x_f = 127.0 * (1.0 - main_x_f);
        } else {
            main_x_f = (1.0 + main_x_f).sqrt();
            main_x_f = 127.0 * (-1.0 + main_x_f);
        }

        if main_y_f >= 0.0 {
            main_y_f = (1.0 - main_y_f).sqrt();
            main_y_f = 127.0 * (1.0 - main_y_f);
        } else {
            main_y_f = (1.0 + main_y_f).sqrt();
            main_y_f = 127.0 * (-1.0 + main_y_f);
        }

        main_x = main_x_f as i32;
        main_y = main_y_f as i32;

        main_x += i32::from(STICK_RANGE.center);
        main_y += i32::from(STICK_RANGE.center);

        input.main_stick.x = main_x.approx_as::<u8>().unwrap_or_saturate();
        input.main_stick.y = main_y.approx_as::<u8>().unwrap_or_saturate();

        input
    }
}

impl mapping::Layer for Vc {
    fn name(&self) -> &'static str {
        "Wii VC OoT"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}
