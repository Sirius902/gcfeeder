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
        let i32_from_bool = |b: bool| if b { 1 } else { 0 };

        let mut u_var4;
        let u_var5;
        let mut i_var6 = n;

        if !(0..=39).contains(&i_var6) && !(-39..=-1).contains(&i_var6) {
            if !(40..=71).contains(&i_var6) {
                if (i_var6 < -39) && (-72 < i_var6) {
                    u_var5 = (-40 - i_var6) * -90;
                    u_var4 = (i_var6 + 72) * -67;
                    u_var4 = (u_var5 >> 5)
                        + i32_from_bool(u_var5 < 0 && (u_var5 & 0x1f) != 0)
                        + (u_var4 >> 5)
                        + i32_from_bool(u_var4 < 0 && (u_var4 & 0x1f) != 0);
                } else if i_var6 < 72 {
                    u_var4 = -90;
                } else {
                    u_var4 = 90;
                }
            } else {
                u_var5 = (72 - i_var6) * 67;
                u_var4 = (i_var6 + -40) * 90;
                u_var4 = (u_var5 >> 5)
                    + i32_from_bool(u_var5 < 0 && (u_var5 & 0x1f) != 0)
                    + (u_var4 >> 5)
                    + i32_from_bool(u_var4 < 0 && (u_var4 & 0x1f) != 0);
            }
        } else {
            i_var6 = (i_var6 * 67) / 40 + ((i_var6 * 67) >> 0x1f);
            u_var4 = i_var6 - (i_var6 >> 0x1f);
        }

        u_var4
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

    fn apply_axis(n: i32) -> i32 {
        todo!()
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
