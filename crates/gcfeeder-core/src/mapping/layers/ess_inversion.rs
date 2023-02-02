use conv::{ConvUtil, UnwrapOrSaturate};
use enum_iterator::Sequence;
use gcinput::{Input, STICK_RANGE};
use serde::{Deserialize, Serialize};

use crate::mapping;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Sequence)]
pub enum EssInversion {
    #[serde(rename = "oot-vc")]
    OotVc,
    #[serde(rename = "mm-vc")]
    MmVc,
    #[serde(rename = "z64-gc")]
    Z64Gc,
}

impl EssInversion {
    pub const fn normalized_map(self) -> &'static NormalizedMap {
        match self {
            Self::OotVc => {
                const MAP: NormalizedMap =
                    NormalizedMap::new(include_bytes!("../../../resource/ess/oot-vc.bin"));
                &MAP
            }
            Self::MmVc => {
                const MAP: NormalizedMap =
                    NormalizedMap::new(include_bytes!("../../../resource/ess/mm-vc.bin"));
                &MAP
            }
            Self::Z64Gc => {
                const MAP: NormalizedMap =
                    NormalizedMap::new(include_bytes!("../../../resource/ess/z64-gc.bin"));
                &MAP
            }
        }
    }

    pub fn apply_scaling(coords: [u8; 2]) -> [u8; 2] {
        gc_to_n64(coords)
    }
}

impl mapping::Layer for EssInversion {
    fn name(&self) -> &'static str {
        match *self {
            Self::OotVc => "OoT VC ESS",
            Self::MmVc => "MM VC ESS",
            Self::Z64Gc => "Z64 GC ESS",
        }
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        let swap = |coords: [u8; 2]| [coords[1], coords[0]];

        input.map(|input| {
            let should_swap = input.main_stick.y > input.main_stick.x;
            let coords: [u8; 2] = input.main_stick.into();

            let (q, mut coords) = Quadrant::normalize(coords);
            if should_swap {
                coords = swap(coords);
            }

            coords = Self::apply_scaling(coords);

            if should_swap {
                coords = swap(coords);
            }

            coords = self.normalized_map().map(coords);
            coords = q.denormalize(coords);

            Input {
                main_stick: coords.into(),
                ..input
            }
        })
    }
}

pub struct NormalizedMap {
    table: &'static [u8; Self::DIM * Self::DIM * 2],
}

impl NormalizedMap {
    const DIM: usize = 128;

    pub const fn new(table: &'static [u8; Self::DIM * Self::DIM * 2]) -> Self {
        Self { table }
    }

    pub fn map(&self, coords: [u8; 2]) -> [u8; 2] {
        let index = 2 * ((usize::from(coords[1]) * Self::DIM) + usize::from(coords[0]));
        [self.table[index], self.table[index + 1]]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Quadrant {
    One,
    Two,
    Three,
    Four,
}

impl Quadrant {
    pub const fn of(coords: [u8; 2]) -> Self {
        #[allow(clippy::collapsible_else_if)]
        if coords[0] >= STICK_RANGE.center {
            if coords[1] >= STICK_RANGE.center {
                Self::One
            } else {
                Self::Four
            }
        } else {
            if coords[1] >= STICK_RANGE.center {
                Self::Two
            } else {
                Self::Three
            }
        }
    }

    pub fn normalize(coords: [u8; 2]) -> (Self, [u8; 2]) {
        let original = Self::of(coords);

        let x = i16::from(coords[0]);
        let y = i16::from(coords[1]);
        let center = i16::from(STICK_RANGE.center);
        let radius = i16::from(STICK_RANGE.radius);

        let x = match original {
            Self::One | Self::Four => x - center,
            Self::Two | Self::Three => (center - x).min(radius),
        };

        let y = match original {
            Self::One | Self::Two => y - center,
            Self::Three | Self::Four => (center - y).min(radius),
        };

        (
            original,
            [
                x.approx_as::<u8>().unwrap_or_saturate(),
                y.approx_as::<u8>().unwrap_or_saturate(),
            ],
        )
    }

    pub fn denormalize(self, coords: [u8; 2]) -> [u8; 2] {
        let x = i16::from(coords[0]);
        let y = i16::from(coords[1]);
        let center = i16::from(STICK_RANGE.center);

        let x = match self {
            Self::One | Self::Four => x + center,
            Self::Two | Self::Three => center - x,
        };

        let y = match self {
            Self::One | Self::Two => y + center,
            Self::Three | Self::Four => center - y,
        };

        [
            x.approx_as::<u8>().unwrap_or_saturate(),
            y.approx_as::<u8>().unwrap_or_saturate(),
        ]
    }
}

fn gc_to_n64(coords: [u8; 2]) -> [u8; 2] {
    let x = f64::from(coords[0]);
    let y = f64::from(coords[1]);

    let scale = (f64::powf((5_f64.mul_add(x, 2.0 * y)) / 525.0, 2.0) * (7.0 * y / 525.0))
        .mul_add(70.0 / 75.0 - 80.0 / 105.0, 80.0 / 105.0);

    [
        ((x * scale).ceil() as u8).min(127),
        ((y * scale).ceil() as u8).min(127),
    ]
}

#[cfg(test)]
mod tests {
    use gcinput::{StickRange, STICK_RANGE};

    use super::{gc_to_n64, Quadrant};

    #[test]
    fn qudrant_works() {
        let StickRange {
            center: c,
            radius: r,
        } = STICK_RANGE;

        let tests = [
            ([c + 0x30, c], [0x30, 0x00], Quadrant::One),
            ([c - 0x20, c + 0x05], [0x20, 0x05], Quadrant::Two),
            ([c, c - 0x01], [0x00, 0x01], Quadrant::Four),
            ([c - 0x01, c + 0x01], [0x01, 0x01], Quadrant::Two),
            ([c - r, c + r], [r, r], Quadrant::Two),
        ];

        for (original, expected, expected_q) in tests.into_iter() {
            let (q, coords) = Quadrant::normalize(original);
            assert_eq!(
                q, expected_q,
                "expected quadrant {q:?} of {original:?} to be {expected_q:?}"
            );
            assert_eq!(
                coords, expected,
                "expected normalized coords {coords:?} to be {expected:?}"
            );
            let coords = q.denormalize(coords);
            assert_eq!(
                coords, original,
                "expected denormalized coords {coords:?} to match original {original:?}"
            );
        }
    }

    #[test]
    fn gc_to_n64_works() {
        let tests = [
            ([0, 0], [0, 0]),
            ([10, 5], [8, 4]),
            ([0x30, 0x00], [0x25, 0x00]),
            ([0x7F, 0x60], [0x7F, 0x7E]),
            ([0x7F, 0x7F], [0x7F, 0x7F]),
            ([60, 60], [51, 51]),
        ];

        for (coord, expected) in tests.into_iter() {
            let mapped = gc_to_n64(coord);
            assert_eq!(
                mapped, expected,
                "expected f({coord:?}) = {expected:?}, was {mapped:?}",
            );
        }
    }
}
