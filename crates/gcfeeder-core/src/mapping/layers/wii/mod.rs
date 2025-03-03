mod clamp;
mod vc;

pub use clamp::*;
pub use vc::*;

#[cfg(test)]
mod tests {
    use gcinput::{Input, Stick, STICK_RANGE};

    use crate::mapping::layers::wii::{Clamp, Vc};

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

    const TOLERANCE: i32 = 1;

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
            let err_x = i32::from(res.x) - i32::from(clamp.x);
            let err_y = i32::from(res.y) - i32::from(clamp.y);

            assert!(
                err_x.abs() <= TOLERANCE && err_y.abs() <= TOLERANCE,
                "Error for mapped {:?} is at most {}, was ({}, {})",
                raw,
                TOLERANCE,
                err_x,
                err_y,
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
            let err_x = i32::from(res.x) - i32::from(vc.x);
            let err_y = i32::from(res.y) - i32::from(vc.y);

            assert!(
                err_x.abs() <= TOLERANCE && err_y.abs() <= TOLERANCE,
                "Error for mapped {:?} is at most {}, was ({}, {})",
                clamp,
                TOLERANCE,
                err_x,
                err_y,
            );
        }
    }
}
