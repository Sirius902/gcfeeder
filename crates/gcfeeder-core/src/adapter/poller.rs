use std::{
    array, io, mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crossbeam::atomic::AtomicCell;
use tracing::warn;

use crate::util::{
    cell_channel::{self, TrySendError},
    AverageTimer,
};

use super::{
    source::{InputListener, InputSource},
    Adapter, Input, Port, Rumble,
};

pub type InputMessage = Option<Input>;

type SenderData = (cell_channel::Sender<InputMessage>, Port);

pub const ERROR_TIMEOUT: Duration = Duration::from_millis(8);

pub struct Poller {
    context: Arc<Context>,
    thread: Option<JoinHandle<()>>,
}

impl Poller {
    pub fn new() -> Self {
        // TODO(Sirius902) Implement.
        todo!()

        // let context = Arc::new(Context::new());
        // let thread = thread::spawn(enclose!((context) move || context.poll_loop()));
        //
        // Self {
        //     context,
        //     thread: Some(thread),
        // }
    }
}

impl InputSource for Poller {
    type Listener = Listener;

    fn average_poll_time(&self) -> Option<Duration> {
        *self.context.average_poll_time.lock().unwrap()
    }

    fn connected(&self) -> bool {
        self.context.connected.load(Ordering::Acquire)
    }

    fn add_listener(&self, port: Port) -> Self::Listener {
        let (sender, receiver) = cell_channel::channel();
        self.context.senders.lock().unwrap().push((sender, port));
        Listener {
            receiver,
            context: self.context.clone(),
            port,
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        self.context.stop_flag.store(true, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            mem::drop(thread.join());
        }
    }
}

struct Context {
    pub stop_flag: AtomicBool,
    pub connected: AtomicBool,
    pub rumble_states: [AtomicCell<Rumble>; Port::COUNT],
    pub senders: Mutex<Vec<SenderData>>,
    pub average_poll_time: Mutex<Option<Duration>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            stop_flag: Default::default(),
            connected: Default::default(),
            rumble_states: Default::default(),
            senders: Default::default(),
            average_poll_time: Default::default(),
        }
    }

    pub async fn poll_loop(&self) {
        let mut adapter: Option<Adapter> = None;
        let mut timer = AverageTimer::start(Duration::from_secs(1));

        while !self.stop_flag.load(Ordering::Acquire) {
            // TODO(Sirius902) Implement with aysnc.
            // let result = {
            //     let adapter = match self.adapter_or_reload(&mut adapter).await {
            //         Ok(a) => a,
            //         Err(e) => {
            //             warn!("Failed to connect to adapter: {}", e);
            //             thread::sleep(ERROR_TIMEOUT);
            //             continue;
            //         }
            //     };
            //
            //     timer.reset();
            //     let (input, rumble) = thread_pool.join(
            //         || self.process_input(adapter),
            //         || self.process_rumble(adapter),
            //     );
            //
            //     input.and(rumble)
            // };
            //
            // match result {
            //     Err(super::Error::Io(e)) if e.kind() == io::ErrorKind::TimedOut => continue,
            //     Err(e) => {
            //         adapter = None;
            //         warn!("Adapter error: {}", e);
            //         continue;
            //     }
            //     _ => (),
            // }

            *self.average_poll_time.lock().unwrap() = Some(timer.lap());
        }

        self.connected.store(false, Ordering::Release);
    }

    async fn process_input(&self, adapter: &Adapter) -> super::Result<()> {
        let inputs = adapter.read_inputs().await?;
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

    async fn process_rumble(&self, adapter: &Adapter) -> super::Result<()> {
        adapter
            .write_rumble(array::from_fn(|i| self.rumble_states[i].load()))
            .await
    }

    async fn adapter_or_reload<'a>(
        &self,
        adapter: &'a mut Option<Adapter>,
    ) -> super::Result<&'a mut Adapter> {
        if let Some(adapter) = adapter {
            Ok(adapter)
        } else {
            self.connected.store(false, Ordering::Release);
            let adapter = Ok(adapter.insert(Adapter::open().await?));
            self.connected.store(true, Ordering::Release);
            adapter
        }
    }
}

pub struct Listener {
    receiver: cell_channel::Receiver<InputMessage>,
    context: Arc<Context>,
    port: Port,
}

impl InputListener for Listener {
    fn port(&self) -> Port {
        self.port
    }

    fn recv(&self) -> Result<InputMessage, cell_channel::RecvError> {
        self.receiver.recv()
    }

    fn recv_deadline(
        &self,
        deadline: Instant,
    ) -> Result<InputMessage, cell_channel::RecvTimeoutError> {
        self.receiver.recv_deadline(deadline)
    }

    fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<InputMessage, cell_channel::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }

    fn try_recv(&self) -> Result<InputMessage, cell_channel::TryRecvError> {
        self.receiver.try_recv()
    }

    fn set_rumble(&self, rumble: Rumble) {
        self.context.rumble_states[self.port.index()].store(rumble);
    }

    fn reset_rumble(&self) {
        self.set_rumble(Rumble::Off)
    }
}
