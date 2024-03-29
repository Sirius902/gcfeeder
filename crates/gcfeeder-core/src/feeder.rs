use std::{
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use enclose::enclose;
use enum_iterator::Sequence;
use gcinput::Input;
use log::warn;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use rusb::UsbContext;
use serde::{Deserialize, Serialize};

use crate::{
    adapter::poller::{self, ERROR_TIMEOUT},
    bridge::{
        self,
        vigem::{Config as ViGEmConfig, ViGEmBridge},
        Error as BridgeError,
    },
    calibration::{SticksCalibration, TriggersCalibration},
    mapping::{
        self,
        layers::{self, AnalogScaling, CenterCalibration, EssInversion},
    },
    util::{
        recent_channel::{self as recent, RecvTimeoutError, TrySendError},
        AverageTimer,
    },
};

type Result<T> = std::result::Result<T, BridgeError>;
type Bridge = dyn bridge::Bridge + Send + Sync;

pub type Callback = dyn FnMut(&Record) + Send;
pub type Sender = recent::Sender<Record>;
pub type Receiver = recent::Receiver<Record>;
pub type CalibrationSender = recent::Sender<Option<Input>>;
pub type CalibrationReceiver = recent::Receiver<Option<Input>>;
pub type Layer = dyn mapping::Layer + Send;

// TODO: Make this come from the poll rate on the adapter.
pub const INPUT_TIMEOUT: Duration = Duration::from_millis(8);

pub struct Feeder<T: UsbContext + 'static> {
    context: Arc<Context<T>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl<T: UsbContext + 'static> Feeder<T> {
    pub fn new(config: Config, listener: poller::Listener<T>) -> Self {
        let internal_layers: Vec<Box<Layer>> = vec![Box::<CenterCalibration>::default()];
        let mut layers: Vec<Box<Layer>> = Vec::new();

        if (config.analog_scale.abs() - 1.0).abs() >= 1e-10 {
            layers.push(Box::new(AnalogScaling::new(config.analog_scale)));
        }

        if let Some(map) = config.ess.inversion_mapping {
            layers.push(Box::new(map));
        }

        if config.calibration.enabled {
            layers.push(Box::new(layers::Calibration::new(
                config.calibration.stick_data,
                config.calibration.trigger_data,
            )));
        }

        let context = Arc::new(Context::new(config, listener));
        let thread = Some(thread::spawn(
            enclose!((context) move || context.feed_loop(config.rumble, internal_layers, layers)),
        ));

        Self { context, thread }
    }

    #[must_use]
    pub fn average_feed_time(&self) -> Option<Duration> {
        *self.context.average_feed_time.lock().unwrap()
    }

    pub fn on_feed(&self, callback: impl FnMut(&Record) + Send + 'static) {
        let mut callbacks = self.context.callbacks.lock().unwrap();
        callbacks.push(Box::new(callback));
    }

    pub fn send_on_feed(&self, sender: Sender) {
        let mut senders = self.context.senders.lock().unwrap();
        senders.push(sender);
    }

    #[must_use]
    pub fn connected(&self) -> bool {
        self.context.connected.load(Ordering::Acquire)
    }

    pub fn start_calibration(&self, sender: CalibrationSender) {
        *self.context.calibration_sender.lock().unwrap() = Some(sender);
    }
}

impl<T: UsbContext> Drop for Feeder<T> {
    fn drop(&mut self) {
        self.context.stop_flag.store(true, Ordering::Release);

        if let Some(handle) = self.thread.take() {
            mem::drop(handle.join());
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Record {
    pub raw_input: Option<Input>,
    pub layered_input: Option<Input>,
    pub feed_time: Duration,
}

struct Context<T: UsbContext> {
    pub config: Config,
    pub listener: poller::Listener<T>,
    pub stop_flag: AtomicBool,
    pub connected: AtomicBool,
    pub calibration_sender: Mutex<Option<CalibrationSender>>,
    pub callbacks: Mutex<Vec<Box<Callback>>>,
    pub senders: Mutex<Vec<Sender>>,
    pub average_feed_time: Mutex<Option<Duration>>,
    pub thread_pool: rayon::ThreadPool,
}

impl<T: UsbContext> Context<T> {
    pub fn new(config: Config, listener: poller::Listener<T>) -> Self {
        Self {
            config,
            listener,
            stop_flag: Default::default(),
            connected: Default::default(),
            callbacks: Default::default(),
            calibration_sender: Default::default(),
            senders: Default::default(),
            average_feed_time: Default::default(),
            thread_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .unwrap(),
        }
    }

    pub fn feed_loop(
        &self,
        rumble: RumbleSetting,
        mut internal_layers: Vec<Box<Layer>>,
        mut layers: Vec<Box<Layer>>,
    ) {
        let mut bridge: Option<Box<Bridge>> = None;
        let mut timer = AverageTimer::start(0.9).unwrap();

        while !self.stop_flag.load(Ordering::Acquire) {
            let record = {
                let bridge = match self.bridge_or_reload(&mut bridge) {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("Failed to connect to bridge: {}", e);
                        thread::sleep(ERROR_TIMEOUT);
                        continue;
                    }
                };

                timer.reset();

                match rumble {
                    RumbleSetting::On => {
                        self.listener.set_rumble(bridge.rumble_state());
                    }
                    RumbleSetting::Off => {}
                }

                bridge.notify_rumble_consumed();

                match self.listener.recv_timeout(INPUT_TIMEOUT) {
                    Ok(input) => {
                        let apply_layers = |input: Option<Input>, layers: &mut [Box<Layer>]| {
                            layers
                                .iter_mut()
                                .fold(input, |input, layer| layer.apply(input))
                        };

                        let input = apply_layers(input, &mut internal_layers);
                        let layered = apply_layers(input, &mut layers);

                        let (input, layered) = {
                            let mut calibration_sender = self.calibration_sender.lock().unwrap();

                            if let Some(sender) = calibration_sender.as_ref() {
                                if let Err(TrySendError::Disconnected(_)) = sender.try_send(input) {
                                    *calibration_sender = None;
                                }

                                (Some(Input::default()), Some(Input::default()))
                            } else {
                                (input, layered)
                            }
                        };

                        bridge.feed(&layered).map(|()| Record {
                            raw_input: input,
                            layered_input: layered,
                            feed_time: timer.read(),
                        })
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                    _ => continue,
                }
            };

            match record {
                Ok(record) => {
                    {
                        self.thread_pool.join(
                            || {
                                let mut callbacks = self.callbacks.lock().unwrap();
                                callbacks
                                    .par_iter_mut()
                                    .for_each(|callback| callback(&record));
                            },
                            || {
                                let mut senders = self.senders.lock().unwrap();

                                senders.retain(|sender| {
                                    !matches!(
                                        sender.try_send(record),
                                        Err(TrySendError::Disconnected(_))
                                    )
                                });
                            },
                        );
                    }

                    *self.average_feed_time.lock().unwrap() = Some(timer.lap());
                }
                Err(e) => {
                    bridge = None;
                    warn!("Bridge error: {}", e);
                    continue;
                }
            }
        }

        self.connected.store(false, Ordering::Release);
    }

    fn bridge_or_reload<'a>(&self, bridge: &'a mut Option<Box<Bridge>>) -> Result<&'a mut Bridge> {
        if let Some(bridge) = bridge {
            Ok(bridge.as_mut())
        } else {
            self.connected.store(false, Ordering::Release);
            let b = match self.config.driver {
                Driver::ViGEm => Box::new(ViGEmBridge::new(
                    self.config.vigem_config,
                    vigem_client::Client::connect()?,
                )?),
            };
            self.connected.store(true, Ordering::Release);

            Ok(bridge.insert(b).as_mut())
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Config {
    pub driver: Driver,
    pub rumble: RumbleSetting,
    pub analog_scale: f64,
    pub vigem_config: ViGEmConfig,
    pub calibration: CalibrationConfig,
    pub ess: EssConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            driver: Default::default(),
            rumble: Default::default(),
            analog_scale: 1.0,
            vigem_config: Default::default(),
            calibration: Default::default(),
            ess: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Sequence)]
#[serde(rename_all = "lowercase")]
pub enum Driver {
    ViGEm,
}

impl Default for Driver {
    fn default() -> Self {
        Self::ViGEm
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub enabled: bool,
    pub stick_data: Option<SticksCalibration>,
    pub trigger_data: Option<TriggersCalibration>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EssConfig {
    pub inversion_mapping: Option<EssInversion>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Sequence)]
#[serde(rename_all = "snake_case")]
pub enum RumbleSetting {
    On,
    Off,
}

impl Default for RumbleSetting {
    fn default() -> Self {
        Self::On
    }
}
