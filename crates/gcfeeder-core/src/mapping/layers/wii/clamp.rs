use gcinput::Input;

use crate::mapping;

#[derive(Default)]
pub struct Clamp;

impl Clamp {
    pub const fn new() -> Self {
        Self
    }

    pub fn apply(mut input: Input) -> Input {
        todo!()
    }
}

impl mapping::Layer for Clamp {
    fn name(&self) -> &'static str {
        "Wii Clamp"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        todo!()
    }
}
