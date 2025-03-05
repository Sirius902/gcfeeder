mod clamp;
mod vc;

pub use clamp::*;
use gcinput::{Stick, STICK_RANGE};
pub use vc::*;

pub fn is_ess(stick: Stick) -> bool {
    const DEADZONE: i32 = 7;
    const MAX: i32 = 67;

    let apply_deadzone = |n: i32| {
        if n > DEADZONE {
            if n < MAX {
                n - DEADZONE
            } else {
                MAX - DEADZONE
            }
        } else if n < -DEADZONE {
            if n > -MAX {
                n + DEADZONE
            } else {
                -MAX + DEADZONE
            }
        } else {
            0
        }
    };

    let x = i32::from(stick.x) - i32::from(STICK_RANGE.center);
    let y = i32::from(stick.y) - i32::from(STICK_RANGE.center);

    let rx = apply_deadzone(x);
    let ry = apply_deadzone(y);

    // Not in ESS position if in the deadzone.
    if rx == 0 && ry == 0 {
        return false;
    }

    let mag = ((rx * rx + ry * ry) as f32).sqrt();

    let mut speed = mag - 20.0;
    if speed < 0.0 {
        speed = 0.0;
    } else {
        let temp = 1.0 - (speed * 450.0).cos();
        speed = (temp * temp * 30.0) + 7.0;
    }

    speed <= 0.0
}

#[cfg(test)]
mod tests {
    use gcinput::{Input, Stick, STICK_RANGE};

    use super::{Clamp, Vc};
    use crate::layers::oot_vc::{InverseClamp, InverseVc};

    macro_rules! test_data {
        ($( [$x:expr, $y:expr] ),* ) => {
            (
                $(
                    Stick::new(
                        ($x + STICK_RANGE.center as i32) as u8,
                        ($y + STICK_RANGE.center as i32) as u8,
                    ),
                )*
            )
        };
    }

    const TEST_DATA: &[(Stick, Stick, Stick)] = &[
        test_data!([-128, -128], [-39, -39], [-56, -56]),
        test_data!([-128, -127], [-39, -39], [-56, -56]),
        test_data!([-128, -80], [-48, -28], [-77, -36]),
        test_data!([-128, 0], [-56, 0], [-127, 0]),
        test_data!([0, 15], [0, 0], [0, 0]),
        test_data!([0, 16], [0, 1], [0, 1]),
        test_data!([16, 16], [1, 1], [1, 1]),
        test_data!([74, 81], [37, 42], [52, 63]),
        test_data!([75, 82], [37, 42], [52, 63]),
        test_data!([76, 83], [37, 41], [52, 60]),
        test_data!([0, 95], [0, 56], [0, 127]),
        test_data!([23, 82], [6, 56], [6, 127]),
        test_data!([95, 15], [56, 0], [127, 0]),
        test_data!([96, 15], [56, 0], [127, 0]),
        test_data!([96, -128], [32, -45], [43, -70]),
        test_data!([127, 96], [45, 32], [70, 43]),
    ];

    #[test]
    fn clamp_main_stick_works() {
        for (raw, clamp, _) in TEST_DATA {
            let input = Input {
                main_stick: *raw,
                ..Default::default()
            };

            let res = Clamp::clamp(input).main_stick;
            assert_eq!(
                res, *clamp,
                "Expected {:?} -> {:?}, got {:?}",
                raw, clamp, res
            );
        }
    }

    #[test]
    fn vc_main_stick_works() {
        for (_, clamp, vc) in TEST_DATA {
            let input = Input {
                main_stick: *clamp,
                ..Default::default()
            };

            let res = Vc::apply(input).main_stick;
            assert_eq!(res, *vc, "Expected {:?} -> {:?}, got {:?}", clamp, vc, res);
        }
    }

    #[test]
    fn inv_clamp_main_stick_works() {
        for (_, clamp, _) in TEST_DATA {
            let input = Input {
                main_stick: *clamp,
                ..Default::default()
            };

            let res = Clamp::clamp(InverseClamp::unclamp(input)).main_stick;
            assert_eq!(
                res, *clamp,
                "Expected {:?} -> {:?}, got {:?}",
                clamp, clamp, res
            );
        }
    }

    #[test]
    fn inv_vc_main_stick_works() {
        for (_, _, vc) in TEST_DATA {
            let input = Input {
                main_stick: *vc,
                ..Default::default()
            };

            let res = Vc::apply(InverseVc::apply(input)).main_stick;
            assert_eq!(res, *vc, "Expected {:?} -> {:?}, got {:?}", vc, vc, res);
        }
    }
}
