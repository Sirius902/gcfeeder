use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct StickCalibration {
    pub notch_points: [[u8; 2]; 8],
    pub center: [u8; 2],
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SticksCalibration {
    pub main_stick: StickCalibration,
    pub c_stick: StickCalibration,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TriggerCalibration {
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TriggersCalibration {
    pub left_trigger: TriggerCalibration,
    pub right_trigger: TriggerCalibration,
}
