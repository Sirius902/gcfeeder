use std::array;

use nalgebra::{Matrix3, Vector2, Vector3};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::adapter::{Input, Stick, STICK_RANGE, TRIGGER_RANGE};

pub const NOTCHES: usize = 8;

pub static NOTCH_POINTS: Lazy<[[u8; 2]; NOTCHES]> = Lazy::new(|| {
    use std::f64::consts::{PI, TAU};

    const CENTER: f64 = STICK_RANGE.center as f64;
    const RADIUS: f64 = STICK_RANGE.radius as f64;

    let start_angle = PI * 0.5;
    (0..NOTCHES)
        .map(|i| i as f64 / NOTCHES as f64)
        .map(|t| start_angle - t * TAU)
        .map(|angle| [RADIUS * angle.cos() + CENTER, RADIUS * angle.sin() + CENTER])
        .map(|pos| pos.map(|x| x.round() as u8))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
});

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad calibration")]
    BadCalibration,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickCalibration {
    pub notch_points: [[u8; 2]; NOTCHES],
    pub center: [u8; 2],
}

impl StickCalibration {
    pub fn map(&self, pos: Stick) -> Result<Stick> {
        use std::f32::consts::PI;

        let q = self.quadrant(pos);
        let qn = (q + self.notch_points.len() - 1) % self.notch_points.len();

        let stick_center =
            Vector2::new(f32::from(STICK_RANGE.center), f32::from(STICK_RANGE.center));

        let left_point = Vector2::new(
            f32::from(self.notch_points[q][0]),
            f32::from(self.notch_points[q][1]),
        );
        let right_point = Vector2::new(
            f32::from(self.notch_points[qn][0]),
            f32::from(self.notch_points[qn][1]),
        );

        let theta1 = q as f32 * PI / 4.0;
        let theta2 = qn as f32 * PI / 4.0;

        let d = [
            Vector2::new(f32::from(STICK_RANGE.center), f32::from(STICK_RANGE.center)),
            Vector2::new(
                f32::from(STICK_RANGE.radius) * theta1.cos() + f32::from(STICK_RANGE.center),
                f32::from(STICK_RANGE.radius) * theta1.sin() + f32::from(STICK_RANGE.center),
            ),
            Vector2::new(
                f32::from(STICK_RANGE.radius) * theta2.cos() + f32::from(STICK_RANGE.center),
                f32::from(STICK_RANGE.radius) * theta2.sin() + f32::from(STICK_RANGE.center),
            ),
        ];

        let mut a = Matrix3::new(
            stick_center.x,
            left_point.x,
            right_point.x,
            stick_center.y,
            left_point.y,
            right_point.y,
            1.0,
            1.0,
            1.0,
        );

        let x = Matrix3::new(
            d[0].x, d[1].x, d[2].x, d[0].y, d[1].y, d[2].y, 1.0, 1.0, 1.0,
        );

        if a.try_inverse_mut() {
            let t = x * a;
            let res = t * Vector3::new(f32::from(pos.x), f32::from(pos.y), 1.0);
            Ok(Stick::new(res.y.round() as u8, res.x.round() as u8))
        } else {
            Err(Error::BadCalibration)
        }
    }

    fn quadrant(&self, pos: Stick) -> usize {
        let angles: [f32; NOTCHES] = array::from_fn(|i| {
            let dx = f32::from(self.notch_points[i][0]) - f32::from(self.center[0]);
            let dy = f32::from(self.notch_points[i][1]) - f32::from(self.center[1]);
            dy.atan2(dx)
        });

        let dx = f32::from(pos.x) - f32::from(self.center[0]);
        let dy = f32::from(pos.y) - f32::from(self.center[1]);
        let angle = dy.atan2(dx);

        let max_index = {
            let max = angles
                .iter()
                .reduce(|a, b| if a < b { b } else { a })
                .unwrap();
            angles.iter().position(|&x| x == *max).unwrap()
        };

        let max_angle = angles[max_index];
        let min_angle = angles[(max_index + angles.len() - 1) % angles.len()];

        if angle > max_angle || angle < min_angle {
            return max_index;
        }

        for (i, a) in angles.iter().enumerate() {
            if i == max_index {
                continue;
            }

            let start_angle = *a;
            let end_angle = angles[(i + angles.len() - 1) % angles.len()];

            if angle >= start_angle && angle <= end_angle {
                return i;
            }
        }

        unreachable!();
    }
}

impl Default for StickCalibration {
    fn default() -> Self {
        Self {
            notch_points: *NOTCH_POINTS,
            center: [STICK_RANGE.center, STICK_RANGE.center],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SticksCalibration {
    pub main_stick: StickCalibration,
    pub c_stick: StickCalibration,
}

impl SticksCalibration {
    pub fn map(&self, mut input: Input) -> Result<Input> {
        input.main_stick = self.main_stick.map(input.main_stick)?;
        input.c_stick = self.c_stick.map(input.c_stick)?;
        Ok(input)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerCalibration {
    pub min: u8,
    pub max: u8,
}

impl TriggerCalibration {
    pub fn map(&self, value: u8) -> Result<u8> {
        if self.min >= self.max {
            return Err(Error::BadCalibration);
        }

        let min = f32::from(self.min);
        let max = f32::from(self.max);
        let trigger_min = f32::from(TRIGGER_RANGE.min);
        let trigger_max = f32::from(TRIGGER_RANGE.max);

        let value_norm = (f32::from(value) - min) / (max - min);
        Ok((value_norm * (trigger_max - trigger_min) + trigger_min).round() as u8)
    }
}

impl Default for TriggerCalibration {
    fn default() -> Self {
        Self {
            min: u8::MIN,
            max: u8::MAX,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TriggersCalibration {
    pub left_trigger: TriggerCalibration,
    pub right_trigger: TriggerCalibration,
}

impl TriggersCalibration {
    pub fn map(&self, mut input: Input) -> Result<Input> {
        input.left_trigger = self.left_trigger.map(input.left_trigger)?;
        input.right_trigger = self.right_trigger.map(input.right_trigger)?;
        Ok(input)
    }
}
