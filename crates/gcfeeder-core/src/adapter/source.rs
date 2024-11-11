use std::time::{Duration, Instant};

use gcinput::Rumble;

use super::{poller::InputMessage, Port};
use crate::util::cell_channel;

pub trait InputSource {
    type Listener: InputListener;

    #[must_use]
    fn average_poll_time(&self) -> Option<Duration>;

    #[must_use]
    fn connected(&self) -> bool;

    fn add_listener(&self, port: Port) -> Self::Listener;
}

pub trait InputListener: Sync + Send {
    fn port(&self) -> Port;

    fn recv(&self) -> Result<InputMessage, cell_channel::RecvError>;
    fn recv_deadline(
        &self,
        deadline: Instant,
    ) -> Result<InputMessage, cell_channel::RecvTimeoutError>;
    fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<InputMessage, cell_channel::RecvTimeoutError>;
    fn try_recv(&self) -> Result<InputMessage, cell_channel::TryRecvError>;

    fn set_rumble(&self, rumble: Rumble);
    fn reset_rumble(&self);
}
