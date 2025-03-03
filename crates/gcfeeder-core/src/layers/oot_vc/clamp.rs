use conv::{ConvUtil, UnwrapOrSaturate};
use gcinput::{Input, Stick, STICK_RANGE};

#[derive(Default)]
pub struct Clamp;

impl Clamp {
    pub const fn new() -> Self {
        Self
    }

    pub fn clamp(mut input: Input) -> Input {
        input.main_stick = Self::clamp_stick(input.main_stick, 56, 15);
        input.c_stick = Self::clamp_stick(input.c_stick, 44, 15);

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

    pub fn clamp_stick(stick: Stick, radius: u8, deadzone: u8) -> Stick {
        let mut x = i32::from(stick.x) - i32::from(STICK_RANGE.center);
        let mut y = i32::from(stick.y) - i32::from(STICK_RANGE.center);

        let radius = i32::from(radius);
        let deadzone = i32::from(deadzone);

        if x > -deadzone && x < deadzone {
            x = 0;
        } else if x > 0 {
            x -= deadzone;
        } else {
            x += deadzone;
        }

        if y > -deadzone && y < deadzone {
            y = 0;
        } else if y > 0 {
            y -= deadzone;
        } else {
            y += deadzone;
        }

        let mag_sq = x * x + y * y;
        if mag_sq > radius * radius {
            let mag = (mag_sq as f32).sqrt();
            let scale = radius as f32 / mag;

            x = (x as f32 * scale) as i32;
            y = (y as f32 * scale) as i32;
        }

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
        "Wii Clamp"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        input.map(Self::clamp)
    }
}
