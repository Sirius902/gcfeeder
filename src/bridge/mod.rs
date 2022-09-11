use crate::adapter::{Input, Rumble};

pub mod rumble;
pub mod vigem;

pub type Result<T> = std::result::Result<T, Error>;

pub trait Bridge {
    fn driver_name(&self) -> &'static str;
    fn feed(&self, input: &Option<Input>) -> Result<()>;
    fn rumble_state(&self) -> Rumble;
    fn notify_rumble_consumed(&self);
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("vigem: {0}")]
    ViGEm(#[from] vigem_client::Error),
}
