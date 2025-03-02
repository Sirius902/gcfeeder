mod clamp;
mod vc;

pub use clamp::*;
pub use vc::*;

#[cfg(test)]
mod tests {
    use gcinput::{Input, Stick};

    use crate::mapping::layers::wii::{Clamp, Vc};

    macro_rules! test_data {
        ($( [$x:expr, $y:expr] ),* ) => {
            (
                $(
                    Stick::new(
                        unsafe { std::mem::transmute::<i8, u8>($x) },
                        unsafe { std::mem::transmute::<i8, u8>($y) }
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

            assert_eq!(Clamp::apply(input).main_stick, *clamp);
        }
    }

    #[test]
    fn vc_main_stick_works() {
        for (_, clamp, vc) in TEST_DATA {
            let input = Input {
                main_stick: *clamp,
                ..Default::default()
            };

            assert_eq!(Vc::apply(input).main_stick, *vc);
        }
    }
}
