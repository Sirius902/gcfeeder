#![deny(clippy::all)]
use serde::{Deserialize, Serialize};

pub const STICK_RANGE: StickRange = StickRange {
    center: 0x80,
    radius: 0x7F,
};

pub const TRIGGER_RANGE: AnalogRange = AnalogRange {
    min: 0x00,
    max: 0xFF,
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Rumble {
    Off,
    On,
}

impl Default for Rumble {
    fn default() -> Self {
        Self::Off
    }
}

impl From<Rumble> for u8 {
    fn from(rumble: Rumble) -> Self {
        match rumble {
            Rumble::Off => 0,
            Rumble::On => 1,
        }
    }
}

impl From<bool> for Rumble {
    fn from(b: bool) -> Self {
        if b {
            Self::On
        } else {
            Self::Off
        }
    }
}

pub struct StickRange {
    pub center: u8,
    pub radius: u8,
}

pub struct AnalogRange {
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Stick {
    pub x: u8,
    pub y: u8,
}

impl Stick {
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(u8) -> u8,
    {
        Self::new(f(self.x), f(self.y))
    }

    #[cfg(feature = "nalgebra")]
    pub fn to_vector(self) -> nalgebra::Vector2<u8> {
        nalgebra::Vector2::<u8>::from(self)
    }
}

impl Default for Stick {
    fn default() -> Self {
        Self::new(STICK_RANGE.center, STICK_RANGE.center)
    }
}

#[cfg(feature = "nalgebra")]
impl From<Stick> for nalgebra::Vector2<u8> {
    fn from(stick: Stick) -> Self {
        Self::new(stick.x, stick.y)
    }
}

#[cfg(feature = "nalgebra")]
impl From<nalgebra::Vector2<u8>> for Stick {
    fn from(v: nalgebra::Vector2<u8>) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<Stick> for [u8; 2] {
    fn from(stick: Stick) -> Self {
        [stick.x, stick.y]
    }
}

impl From<[u8; 2]> for Stick {
    fn from(pos: [u8; 2]) -> Self {
        Self::new(pos[0], pos[1])
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Input {
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,

    pub button_left: bool,
    pub button_right: bool,
    pub button_down: bool,
    pub button_up: bool,

    pub button_start: bool,
    pub button_z: bool,
    pub button_r: bool,
    pub button_l: bool,

    pub main_stick: Stick,
    pub c_stick: Stick,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            button_a: false,
            button_b: false,
            button_x: false,
            button_y: false,

            button_left: false,
            button_right: false,
            button_down: false,
            button_up: false,

            button_start: false,
            button_z: false,
            button_r: false,
            button_l: false,

            main_stick: Stick::default(),
            c_stick: Stick::default(),
            left_trigger: TRIGGER_RANGE.min,
            right_trigger: TRIGGER_RANGE.min,
        }
    }
}
