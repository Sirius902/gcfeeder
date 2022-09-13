use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crossbeam::{atomic::AtomicCell, channel};

pub type SendError<T> = channel::SendError<T>;
pub type SendTimeoutError<T> = channel::SendTimeoutError<T>;

pub type RecvError = channel::RecvError;
pub type RecvTimeoutError = channel::RecvTimeoutError;
pub type TryRecvError = channel::TryRecvError;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let msg = Arc::new(AtomicCell::new(None));
    let (sender, receiver) = channel::bounded(1);

    (
        Sender {
            sender,
            msg: msg.clone(),
        },
        Receiver { receiver, msg },
    )
}

pub struct Sender<T> {
    sender: channel::Sender<()>,
    msg: Arc<AtomicCell<Option<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.msg.store(Some(msg));
        self.sender
            .send(())
            .map_err(|_| channel::SendError(self.msg()))
    }

    pub fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
        self.msg.store(Some(msg));
        match self.sender.send_deadline((), deadline) {
            Ok(msg) => Ok(msg),
            Err(e) => self.map_timeout(e),
        }
    }

    pub fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
        self.msg.store(Some(msg));
        match self.sender.send_timeout((), timeout) {
            Ok(msg) => Ok(msg),
            Err(e) => self.map_timeout(e),
        }
    }

    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.msg.store(Some(msg));
        match self.sender.try_send(()) {
            Ok(msg) => Ok(msg),
            Err(e) => self.map_try(e),
        }
    }

    fn msg(&self) -> T {
        self.msg.swap(None).unwrap()
    }

    fn map_timeout(&self, e: channel::SendTimeoutError<()>) -> Result<(), SendTimeoutError<T>> {
        match e {
            channel::SendTimeoutError::Disconnected(()) => {
                Err(SendTimeoutError::Disconnected(self.msg()))
            }
            channel::SendTimeoutError::Timeout(()) => Err(SendTimeoutError::Timeout(self.msg())),
        }
    }

    fn map_try(&self, e: channel::TrySendError<()>) -> Result<(), TrySendError<T>> {
        match e {
            channel::TrySendError::Disconnected(()) => Err(TrySendError::Disconnected(self.msg())),
            channel::TrySendError::Full(()) => Ok(()),
        }
    }
}

pub struct Receiver<T> {
    receiver: channel::Receiver<()>,
    msg: Arc<AtomicCell<Option<T>>>,
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        self.receiver.recv().map(|()| self.msg())
    }

    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
        self.receiver.recv_deadline(deadline).map(|()| self.msg())
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout).map(|()| self.msg())
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.receiver.try_recv().map(|()| self.msg())
    }

    fn msg(&self) -> T {
        self.msg.swap(None).unwrap()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrySendError<T> {
    #[error("sending on a disconnected channel")]
    Disconnected(T),
}
