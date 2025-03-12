mod clamp;
mod gc;

pub use clamp::*;
pub use gc::*;

#[cfg(test)]
mod tests {
    use gcinput::{Input, Stick, STICK_RANGE};

    use super::{Clamp, Gc};
    use crate::layers::z64_gc::{InverseClamp, InverseGc};

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
        test_data!([-128, -128], [-40, -40], [-67, -67]),
        test_data!([-128, -127], [-40, -39], [-67, -65]),
        test_data!([-128, -80], [-49, -28], [-73, -46]),
        test_data!([-128, 0], [-72, 0], [-90, 0]),
        test_data!([0, 15], [0, 0], [0, 0]),
        test_data!([0, 16], [0, 1], [0, 1]),
        test_data!([16, 16], [1, 1], [1, 1]),
        test_data!([74, 81], [37, 41], [61, 66]),
        test_data!([75, 82], [37, 41], [61, 66]),
        test_data!([76, 83], [37, 41], [61, 66]),
        test_data!([0, 95], [0, 72], [0, 90]),
        test_data!([23, 82], [7, 65], [11, 84]),
        test_data!([95, 15], [72, 0], [90, 0]),
        test_data!([96, 15], [72, 0], [90, 0]),
        test_data!([96, -128], [32, -45], [53, -70]),
        test_data!([0, 127], [0, 72], [0, 90]),
        test_data!([127, 96], [45, 32], [70, 53]),
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
    fn gc_main_stick_works() {
        for (_, clamp, gc) in TEST_DATA {
            let input = Input {
                main_stick: *clamp,
                ..Default::default()
            };

            let res = Gc::apply(input).main_stick;
            assert_eq!(res, *gc, "Expected {:?} -> {:?}, got {:?}", clamp, gc, res);
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
    fn inv_gc_main_stick_works() {
        for (_, _, gc) in TEST_DATA {
            let input = Input {
                main_stick: *gc,
                ..Default::default()
            };

            let res = Gc::apply(InverseGc::apply(input)).main_stick;
            assert_eq!(res, *gc, "Expected {:?} -> {:?}, got {:?}", gc, gc, res);
        }
    }
}
