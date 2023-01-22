use enum_dispatch::enum_dispatch;
use gcinput::Input;

pub mod layers;

#[enum_dispatch]
pub trait Layer {
    fn name(&self) -> &'static str;
    fn apply(&mut self, input: Option<Input>) -> Option<Input>;
}

#[enum_dispatch(Layer)]
pub enum LayerImpl {
    AnalogScaling(layers::AnalogScaling),
    Calibration(layers::Calibration),
    CenterCalibration(layers::CenterCalibration),
    EssInversion(layers::EssInversion),
}
