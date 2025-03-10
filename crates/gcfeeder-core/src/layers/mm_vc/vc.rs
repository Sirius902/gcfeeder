use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{Input, STICK_RANGE};

#[derive(Default)]
pub struct Vc;

impl Vc {
    pub const fn new() -> Self {
        Self
    }

    pub fn apply(mut input: Input) -> Input {
        let mut main_x = i32::from(input.main_stick.x) - i32::from(STICK_RANGE.center);
        let mut main_y = i32::from(input.main_stick.y) - i32::from(STICK_RANGE.center);

        // c_stick divisor is 59.0
        main_x = (80.0 * (main_x as f64 / 72.0)) as i32;
        main_y = (80.0 * (main_y as f64 / 72.0)) as i32;

        Self::scale_diagonal(&mut main_x, &mut main_y, 0.8);

        main_x += i32::from(STICK_RANGE.center);
        main_y += i32::from(STICK_RANGE.center);

        input.main_stick.x = main_x.approx_as::<u8>().unwrap_or_saturate();
        input.main_stick.y = main_y.approx_as::<u8>().unwrap_or_saturate();

        input
    }

    fn scale_diagonal(x: &mut i32, y: &mut i32, scale: f64) {
        if *x == 0 || *y == 0 {
            return;
        }

        let fx = f64::from(*x);
        let fy = f64::from(*y);

        let mut abs_min = fx.abs();
        let mut abs_max = fy.abs();

        if abs_min > abs_max {
            std::mem::swap(&mut abs_min, &mut abs_max);
        }

        let ratio = abs_min / abs_max;

        *x = (fx * (1.0 + ratio * scale)) as i32;
        *y = (fy * (1.0 + ratio * scale)) as i32;
    }
}

impl crate::layers::Layer for Vc {
    fn name(&self) -> &'static str {
        "mm-vc"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}

#[derive(Default)]
pub struct InverseVc;

impl InverseVc {
    pub const fn new() -> Self {
        Self
    }

    pub fn apply(mut input: Input) -> Input {
        let mut main_x = i32::from(input.main_stick.x) - i32::from(STICK_RANGE.center);
        let mut main_y = i32::from(input.main_stick.y) - i32::from(STICK_RANGE.center);

        Self::scale_diagonal(&mut main_x, &mut main_y, 0.8);

        // c_stick divisor is 59.0
        main_x = (72.0 * (main_x as f64 / 80.0)) as i32;
        main_y = (72.0 * (main_y as f64 / 80.0)) as i32;

        main_x += i32::from(STICK_RANGE.center);
        main_y += i32::from(STICK_RANGE.center);

        input.main_stick.x = main_x.approx_as::<u8>().unwrap_or_saturate();
        input.main_stick.y = main_y.approx_as::<u8>().unwrap_or_saturate();

        input
    }

    fn scale_diagonal(x: &mut i32, y: &mut i32, scale: f64) {
        if *x == 0 || *y == 0 {
            return;
        }

        let fx = f64::from(*x);
        let fy = f64::from(*y);

        let mut abs_min = fx.abs();
        let mut abs_max = fy.abs();

        if abs_min > abs_max {
            std::mem::swap(&mut abs_min, &mut abs_max);
        }

        let ratio = abs_min / abs_max;

        *x = (fx / (1.0 + ratio * scale)) as i32;
        *y = (fy / (1.0 + ratio * scale)) as i32;
    }
}

impl crate::layers::Layer for InverseVc {
    fn name(&self) -> &'static str {
        "Inverse mm-vc"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}
