use gcinput::Input;

use crate::mapping;

#[derive(Default)]
pub struct Vc;

impl Vc {
    pub const fn new() -> Self {
        Self
    }

    pub fn apply(mut input: Input) -> Input {
        todo!()
    }
}

impl mapping::Layer for Vc {
    fn name(&self) -> &'static str {
        "Wii VC"
    }

    fn apply(&mut self, input: Option<Input>) -> Option<Input> {
        todo!()
    }
}
