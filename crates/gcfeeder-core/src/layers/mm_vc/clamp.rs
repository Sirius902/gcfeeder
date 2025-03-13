use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{Input, Stick, STICK_RANGE};

// FUTURE(Sirius902) Merge this module with the z64_gc one, they are identical.

#[derive(Default)]
pub struct Clamp;

impl Clamp {
    pub const fn new() -> Self {
        Self
    }

    pub fn clamp(mut input: Input) -> Input {
        input.main_stick = Self::clamp_stick(input.main_stick, 72, 40, 15);
        input.c_stick = Self::clamp_stick(input.c_stick, 59, 31, 15);

        if input.left_trigger <= 30 {
            input.left_trigger = 0;
        } else if input.left_trigger > 180 {
            input.left_trigger = 180;
        } else {
            input.left_trigger -= 30;
        }

        if input.right_trigger <= 30 {
            input.right_trigger = 0;
        } else if input.right_trigger > 180 {
            input.right_trigger = 180;
        } else {
            input.right_trigger -= 30;
        }

        input
    }

    pub fn clamp_stick(stick: Stick, max: u8, min: u8, deadzone: u8) -> Stick {
        let mut x = i32::from(stick.x) - i32::from(STICK_RANGE.center);
        let mut y = i32::from(stick.y) - i32::from(STICK_RANGE.center);

        let max = i32::from(max);
        let min = i32::from(min);
        let deadzone = i32::from(deadzone);

        let sign_x = x.signum();
        let sign_y = y.signum();

        let mut abs_x = x.abs();
        let mut abs_y = y.abs();

        if abs_x == 0 && abs_y == 0 {
            return Stick::default();
        }

        if abs_x > deadzone {
            abs_x -= deadzone;
        } else {
            abs_x = 0;
        }

        if abs_y > deadzone {
            abs_y -= deadzone;
        } else {
            abs_y = 0;
        }

        let i_var1 = min * max;
        if abs_x < abs_y {
            let i_var2 = min * abs_y + abs_x * (max - min);
            if i_var2 > i_var1 {
                abs_x = (abs_x * i_var1) / i_var2;
                abs_y = (abs_y * i_var1) / i_var2;
            }
        } else {
            let i_var2 = min * abs_x + abs_y * (max - min);
            if i_var2 > i_var1 {
                abs_x = (abs_x * i_var1) / i_var2;
                abs_y = (abs_y * i_var1) / i_var2;
            }
        }

        x = sign_x * abs_x;
        y = sign_y * abs_y;

        x += i32::from(STICK_RANGE.center);
        y += i32::from(STICK_RANGE.center);

        Stick::new(
            x.approx_as::<u8>().unwrap_or_saturate(),
            y.approx_as::<u8>().unwrap_or_saturate(),
        )
    }
}

impl crate::layers::Layer for Clamp {
    fn name(&self) -> &'static str {
        "mm-vc Clamp"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::clamp)
    }
}

#[derive(Default)]
pub struct InverseClamp;

impl InverseClamp {
    pub const fn new() -> Self {
        Self
    }

    pub fn unclamp(mut input: Input) -> Input {
        input.main_stick = Self::unclamp_stick(input.main_stick, 72, 40, 15);
        input.c_stick = Self::unclamp_stick(input.c_stick, 59, 31, 15);

        if input.left_trigger > 0 {
            input.left_trigger += 30;
        }

        if input.right_trigger > 0 {
            input.right_trigger += 30;
        }

        input
    }

    pub fn unclamp_stick(stick: Stick, max: u8, min: u8, deadzone: u8) -> Stick {
        let mut x = i32::from(stick.x) - i32::from(STICK_RANGE.center);
        let mut y = i32::from(stick.y) - i32::from(STICK_RANGE.center);

        let max = i32::from(max);
        let min = i32::from(min);
        let deadzone = i32::from(deadzone);

        let sign_x = x.signum();
        let sign_y = y.signum();

        let mut abs_x = x.abs();
        let mut abs_y = y.abs();

        if abs_x == 0 && abs_y == 0 {
            return Stick::default();
        }

        let i_var1 = min * max;
        if abs_x < abs_y {
            let i_var2 = min * abs_y + abs_x * (max - min);
            if i_var2 > i_var1 {
                abs_x = (abs_x * i_var2) / i_var1;
                abs_y = (abs_y * i_var2) / i_var1;
            }
        } else {
            let i_var2 = min * abs_x + abs_y * (max - min);
            if i_var2 > i_var1 {
                abs_x = (abs_x * i_var2) / i_var1;
                abs_y = (abs_y * i_var2) / i_var1;
            }
        }

        if abs_x > 0 {
            abs_x += deadzone;
        }

        if abs_y > 0 {
            abs_y += deadzone;
        }

        x = sign_x * abs_x;
        y = sign_y * abs_y;

        x += i32::from(STICK_RANGE.center);
        y += i32::from(STICK_RANGE.center);

        Stick::new(
            x.approx_as::<u8>().unwrap_or_saturate(),
            y.approx_as::<u8>().unwrap_or_saturate(),
        )
    }
}

impl crate::layers::Layer for InverseClamp {
    fn name(&self) -> &'static str {
        "Inverse mm-vc Clamp"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::unclamp)
    }
}
