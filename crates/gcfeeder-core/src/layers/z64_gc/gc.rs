use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{Input, STICK_RANGE};

#[derive(Default)]
pub struct Gc;

impl Gc {
    pub const fn new() -> Self {
        Self
    }

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
        match n {
            -39..=39 => (n * 67) / 40,
            -71..=-40 => {
                let n1 = (-40 - n) * -90;
                let n2 = (n + 72) * -67;
                n1 / 32 + n2 / 32
            }
            40..=71 => {
                let n1 = (72 - n) * 67;
                let n2 = (n - 40) * 90;
                n1 / 32 + n2 / 32
            }
            ..=-72 => -90,
            72.. => 90,
        }
    }
}

impl crate::layers::Layer for Gc {
    fn name(&self) -> &'static str {
        "z64-gc"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}

#[derive(Default)]
pub struct InverseGc;

impl InverseGc {
    pub const fn new() -> Self {
        Self
    }

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

    // TODO(Sirius902) Make this more accurate.
    fn apply_axis(n: i32) -> i32 {
        match n {
            -90 => -72,
            90 => 72,
            -89..=-68 => {
                let mut lo = -72;
                let mut hi = -40;
                while lo < hi {
                    let mid = (lo + hi + 1) / 2;
                    if Gc::apply_axis(mid) < n {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }
                lo
            }
            68..=89 => {
                let mut lo = 40;
                let mut hi = 72;
                while lo < hi {
                    let mid = (lo + hi) / 2;
                    if Gc::apply_axis(mid) > n {
                        hi = mid;
                    } else {
                        lo = mid + 1;
                    }
                }
                lo - 1
            }
            _ => ((n - ((n >> 31) - ((n / 67) >> 31))) * 40) / 67,
        }
    }
}

impl crate::layers::Layer for InverseGc {
    fn name(&self) -> &'static str {
        "Inverse z64-gc"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::apply)
    }
}
