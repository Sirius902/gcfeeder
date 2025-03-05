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

        main_x = (127.0 * (main_x as f64 / 56.0)) as i32;
        main_y = (127.0 * (main_y as f64 / 56.0)) as i32;

        let mut main_x_f = main_x as f64 / 127.0;
        if main_x_f >= 0.0 {
            main_x_f = (1.0 - main_x_f).sqrt();
            main_x_f = 127.0 * (1.0 - main_x_f);
        } else {
            main_x_f = (1.0 + main_x_f).sqrt();
            main_x_f = 127.0 * (-1.0 + main_x_f);
        }

        let mut main_y_f = main_y as f64 / 127.0;
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

impl crate::layers::Layer for Vc {
    fn name(&self) -> &'static str {
        "oot-vc"
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

    // https://www.desmos.com/calculator/69wkbm1jsv
    pub fn apply(mut input: Input) -> Input {
        let mut main_x = i32::from(input.main_stick.x) - i32::from(STICK_RANGE.center);
        let mut main_y = i32::from(input.main_stick.y) - i32::from(STICK_RANGE.center);

        main_x = Self::apply_axis(main_x);
        main_y = Self::apply_axis(main_y);

        main_x += i32::from(STICK_RANGE.center);
        main_y += i32::from(STICK_RANGE.center);

        input.main_stick.x = main_x.approx_as::<u8>().unwrap_or_saturate();
        input.main_stick.y = main_y.approx_as::<u8>().unwrap_or_saturate();

        input
    }

    fn apply_axis(n: i32) -> i32 {
        let a = |x: f64| if x >= 0.0 { x.ceil() } else { x.floor() };

        let q = |x: f64| {
            if x >= 0.0 {
                2.0 * x - (x * x)
            } else {
                x * x + 2.0 * x
            }
        };

        let h = |x: f64| (56.0 / 127.0) * a(127.0 * q(a(x) / 127.0));

        let d = 0.2;
        let x = n as f64;

        ((h(x - d) + h(x + d)) / 2.0).round() as i32
    }
}

impl crate::layers::Layer for InverseVc {
    fn name(&self) -> &'static str {
        "Inverse oot-vc"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}
