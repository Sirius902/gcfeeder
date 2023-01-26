use std::{
    array, mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossbeam::atomic::AtomicCell;
use enclose::enclose;
use log::warn;
use rusb::UsbContext;

use crate::util::{
    recent_channel::{self as recent, TrySendError},
    AverageTimer,
};

use super::{source::InputListener, Adapter, Input, Port, Rumble};

pub type InputMessage = Option<Input>;

type SenderData = (recent::Sender<InputMessage>, Port);

pub const ERROR_TIMEOUT: Duration = Duration::from_millis(8);

pub struct Poller<T: UsbContext + 'static> {
    context: Arc<Context<T>>,
    thread: Option<JoinHandle<()>>,
}

impl<T: UsbContext + 'static> Poller<T> {
    pub fn new(usb_context: T) -> Self {
        let context = Arc::new(Context::new(usb_context));
        let thread = thread::spawn(enclose!((context) move || context.poll_loop()));

        Self {
            context,
            thread: Some(thread),
        }
    }

    #[must_use]
    pub fn average_poll_time(&self) -> Option<Duration> {
        *self.context.average_poll_time.lock().unwrap()
    }

    #[must_use]
    pub fn connected(&self) -> bool {
        self.context.connected.load(Ordering::Acquire)
    }

    pub fn add_listener(&self, port: Port) -> Listener<T> {
        let (sender, receiver) = recent::channel();
        self.context.senders.lock().unwrap().push((sender, port));
        Listener {
            receiver,
            context: self.context.clone(),
            port,
        }
    }
}

impl<T: UsbContext> Drop for Poller<T> {
    fn drop(&mut self) {
        self.context.stop_flag.store(true, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            mem::drop(thread.join());
        }
    }
}

struct Context<T: UsbContext> {
    pub stop_flag: AtomicBool,
    pub connected: AtomicBool,
    pub usb_context: T,
    pub rumble_states: [AtomicCell<Rumble>; Port::COUNT],
    pub senders: Mutex<Vec<SenderData>>,
    pub average_poll_time: Mutex<Option<Duration>>,
}

impl<T: UsbContext> Context<T> {
    pub fn new(usb_context: T) -> Self {
        Self {
            stop_flag: Default::default(),
            connected: Default::default(),
            usb_context,
            rumble_states: Default::default(),
            senders: Default::default(),
            average_poll_time: Default::default(),
        }
    }

    pub fn poll_loop(&self) {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let mut adapter: Option<Adapter<T>> = None;
        let mut timer = AverageTimer::start(Duration::from_secs(1));

        while !self.stop_flag.load(Ordering::Acquire) {
            let result = {
                let adapter = match self.adapter_or_reload(&mut adapter) {
                    Ok(a) => a,
                    Err(e) => {
                        warn!("Failed to connect to adapter: {}", e);
                        thread::sleep(ERROR_TIMEOUT);
                        continue;
                    }
                };

                timer.reset();
                let (input, rumble) = thread_pool.join(
                    || self.process_input(adapter),
                    || self.process_rumble(adapter),
                );

                input.and(rumble)
            };

            match result {
                Err(super::Error::Usb(rusb::Error::Timeout)) => continue,
                Err(e) => {
                    adapter = None;
                    warn!("Adapter error: {}", e);
                    continue;
                }
                _ => (),
            }

            *self.average_poll_time.lock().unwrap() = Some(timer.lap());
        }

        self.connected.store(false, Ordering::Release);
    }

    fn process_input(&self, adapter: &Adapter<T>) -> super::Result<()> {
        let inputs = adapter.read_inputs()?;
        let mut senders = self.senders.lock().unwrap();

        senders.retain(|(sender, port)| {
            let index = port.index();
            !matches!(
                sender.try_send(inputs[index]),
                Err(TrySendError::Disconnected(_))
            )
        });

        Ok(())
    }

    fn process_rumble(&self, adapter: &Adapter<T>) -> super::Result<()> {
        adapter.write_rumble(array::from_fn(|i| self.rumble_states[i].load()))
    }

    fn adapter_or_reload<'a>(
        &self,
        adapter: &'a mut Option<Adapter<T>>,
    ) -> super::Result<&'a mut Adapter<T>> {
        if let Some(adapter) = adapter {
            Ok(adapter)
        } else {
            self.connected.store(false, Ordering::Release);
            let adapter = Ok(adapter.insert(Adapter::open(&self.usb_context)?));
            self.connected.store(true, Ordering::Release);
            adapter
        }
    }
}

pub struct Listener<T: UsbContext> {
    receiver: recent::Receiver<InputMessage>,
    context: Arc<Context<T>>,
    port: Port,
}

impl<T: UsbContext> InputListener for Listener<T> {
    fn port(&self) -> Port {
        self.port
    }

    fn recv(&self) -> Result<InputMessage, recent::RecvError> {
        self.receiver.recv()
    }

    fn recv_deadline(&self, deadline: Instant) -> Result<InputMessage, recent::RecvTimeoutError> {
        self.receiver.recv_deadline(deadline)
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<InputMessage, recent::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    fn try_recv(&self) -> Result<InputMessage, recent::TryRecvError> {
        self.receiver.try_recv()
    }

    fn set_rumble(&self, rumble: Rumble) {
        self.context.rumble_states[self.port.index()].store(rumble);
    }

    fn reset_rumble(&self) {
        self.set_rumble(Rumble::Off)
    }
}
