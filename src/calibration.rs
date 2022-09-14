use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::adapter::STICK_RANGE;

pub const NOTCHES: usize = 8;

pub static NOTCH_POINTS: Lazy<[[u8; 2]; NOTCHES]> = Lazy::new(|| {
    use std::f64::consts::{PI, TAU};

    const CENTER: f64 = STICK_RANGE.center as f64;
    const RADIUS: f64 = STICK_RANGE.radius as f64;

    let start_angle = PI * 0.5;
    (0..NOTCHES)
        .map(|i| i as f64 / NOTCHES as f64)
        .map(|t| start_angle + t * TAU)
        .map(|angle| [RADIUS * angle.cos() + CENTER, RADIUS * angle.sin() + CENTER])
        .map(|pos| pos.map(|x| x as u8))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
});

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct StickCalibration {
    pub notch_points: [[u8; 2]; NOTCHES],
    pub center: [u8; 2],
}

impl Default for StickCalibration {
    fn default() -> Self {
        Self {
            notch_points: *NOTCH_POINTS,
            center: [STICK_RANGE.center, STICK_RANGE.center],
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct SticksCalibration {
    pub main_stick: StickCalibration,
    pub c_stick: StickCalibration,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TriggerCalibration {
    pub min: u8,
    pub max: u8,
}

impl Default for TriggerCalibration {
    fn default() -> Self {
        Self {
            min: u8::MIN,
            max: u8::MAX,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct TriggersCalibration {
    pub left_trigger: TriggerCalibration,
    pub right_trigger: TriggerCalibration,
}
