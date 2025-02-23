use gcinput::Input;

pub mod layers;

pub trait Layer: Send {
    fn name(&self) -> &'static str;
    fn apply(&mut self, input: Option<Input>) -> Option<Input>;
}
