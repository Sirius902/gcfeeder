use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use bridge::Bridge;
use crossbeam::atomic::AtomicCell;
use gcinput::Input;
use mapping::Layer;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::warn;

use crate::{
    adapter::{poller::ERROR_TIMEOUT, source::InputListener},
    bridge::{self, Driver, Error as BridgeError},
    calibration::{SticksCalibration, TriggersCalibration},
    mapping::{
        self,
        layers::{self, AnalogScaling, CenterCalibration, EssInversion},
    },
    util::{
        cell_channel::{self, RecvTimeoutError, TrySendError},
        AverageTimer,
    },
};

#[cfg(target_os = "windows")]
use crate::bridge::vigem::Config as ViGEmConfig;

type Result<T> = std::result::Result<T, BridgeError>;

pub type Callback = dyn FnMut(&Record) + Send;
pub type Sender = cell_channel::Sender<Record>;
pub type Receiver = cell_channel::Receiver<Record>;

// FUTURE(Sirius902) Make this come from the poll rate on the adapter.
pub const INPUT_TIMEOUT: Duration = Duration::from_millis(8);

pub struct Feeder<L: InputListener + 'static> {
    context: Arc<Context<L>>,
    task: Option<JoinHandle<()>>,
}

impl<L: InputListener + 'static> Feeder<L> {
    pub fn new(config: Config, input_source: L) -> Self {
        let internal_layers: Vec<Box<dyn Layer>> = vec![Box::new(CenterCalibration::default())];
        let mut layers: Vec<Box<dyn Layer>> = Vec::new();

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

        let context = Arc::new(Context::new(config, input_source));
        let task = Some(tokio::task::spawn({
            let context = context.clone();
            async move {
                context
                    .feed_loop(config.rumble, internal_layers, layers)
                    .await
            }
        }));

        Self { context, task }
    }

    pub async fn close(mut self) {
        self.context.stop_flag.store(true, Ordering::Release);

        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }

    #[must_use]
    pub async fn average_feed_time(&self) -> Option<Duration> {
        *self.context.average_feed_time.lock().await
    }

    pub async fn on_feed(&self, callback: impl FnMut(&Record) + Send + 'static) {
        let mut callbacks = self.context.callbacks.lock().await;
        callbacks.push(Box::new(callback));
    }

    pub async fn send_on_feed(&self, sender: Sender) {
        let mut senders = self.context.senders.lock().await;
        senders.push(sender);
    }

    #[must_use]
    pub fn connected(&self) -> bool {
        self.context.connected.load(Ordering::Acquire)
    }

    pub async fn start_calibration(&self, sender: CalibrationSender) {
        *self.context.calibration_sender.lock().await = Some(sender);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Record {
    pub raw_input: Option<Input>,
    pub layered_input: Option<Input>,
    pub feed_time: Duration,
}

struct Context<L: InputListener> {
    pub config: Config,
    pub input_source: L,
    pub stop_flag: AtomicBool,
    pub connected: AtomicBool,
    pub calibration_sender: Mutex<Option<CalibrationSender>>,
    pub callbacks: Mutex<Vec<Box<Callback>>>,
    pub senders: Mutex<Vec<Sender>>,
    pub average_feed_time: Mutex<Option<Duration>>,
}

impl<L: InputListener> Context<L> {
    pub fn new(config: Config, input_source: L) -> Self {
        Self {
            config,
            input_source,
            stop_flag: Default::default(),
            connected: Default::default(),
            callbacks: Default::default(),
            calibration_sender: Default::default(),
            senders: Default::default(),
            average_feed_time: Default::default(),
        }
    }

    pub async fn feed_loop(
        &self,
        rumble: RumbleSetting,
        mut internal_layers: Vec<Box<dyn Layer>>,
        mut layers: Vec<Box<dyn Layer>>,
    ) {
        let mut bridge: Option<Box<dyn Bridge>> = None;
        let mut timer = AverageTimer::start(Duration::from_secs(1));

        while !self.stop_flag.load(Ordering::Acquire) {
            let record = {
                let bridge = match self.bridge_or_reload(&mut bridge) {
                    Ok(b) => b,
                    Err(e) => {
                        warn!("Failed to connect to bridge: {}", e);
                        tokio::time::sleep(ERROR_TIMEOUT).await;
                        continue;
                    }
                };

                timer.reset();

                match rumble {
                    RumbleSetting::On => {
                        self.input_source.set_rumble(bridge.rumble_state());
                    }
                    RumbleSetting::Off => {}
                }

                bridge.notify_rumble_consumed();

                match self.input_source.recv_timeout(INPUT_TIMEOUT) {
                    Ok(input) => {
                        let apply_layers = |input: Option<Input>, layers: &mut [Box<dyn Layer>]| {
                            layers
                                .iter_mut()
                                .fold(input, |input, layer| layer.apply(input))
                        };

                        let input = apply_layers(input, &mut internal_layers);
                        let layered = apply_layers(input, &mut layers);

                        let (input, layered) = {
                            let mut calibration_sender = self.calibration_sender.lock().await;

                            if let Some(sender) = calibration_sender.as_ref() {
                                if let Err(CalibrationDisconnected) = sender.try_send(input) {
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
                        let mut callbacks = self.callbacks.lock().await;
                        callbacks.iter_mut().for_each(|callback| callback(&record));
                    }

                    {
                        let mut senders = self.senders.lock().await;
                        senders.retain(|sender| {
                            !matches!(sender.try_send(record), Err(TrySendError::Disconnected(_)))
                        });
                    }

                    *self.average_feed_time.lock().await = Some(timer.lap());
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

    fn bridge_or_reload<'a>(
        &self,
        bridge: &'a mut Option<Box<dyn Bridge>>,
    ) -> Result<&'a mut Box<dyn Bridge>> {
        if let Some(bridge) = bridge {
            Ok(bridge)
        } else {
            self.connected.store(false, Ordering::Release);
            let b = self.config.driver.create_bridge(&self.config)?;
            self.connected.store(true, Ordering::Release);

            Ok(bridge.insert(b))
        }
    }
}

#[derive(Copy, Clone)]
enum CalibrationState {
    Connected(Option<Input>),
    Disconnected,
}

#[derive(Debug, thiserror::Error)]
#[error("calibration disconnected")]
pub struct CalibrationDisconnected;

/// Creates an `Input` channel where the receiver will always yield the most recently received message.
pub fn calibration_channel() -> (CalibrationSender, CalibrationReceiver) {
    let state = Arc::new(AtomicCell::new(CalibrationState::Connected(None)));
    (
        CalibrationSender {
            state: state.clone(),
        },
        CalibrationReceiver { state },
    )
}

pub struct CalibrationSender {
    state: Arc<AtomicCell<CalibrationState>>,
}

impl CalibrationSender {
    pub fn try_send(
        &self,
        input: Option<Input>,
    ) -> std::result::Result<(), CalibrationDisconnected> {
        if matches!(self.state.load(), CalibrationState::Disconnected) {
            Err(CalibrationDisconnected)
        } else {
            self.state.store(CalibrationState::Connected(input));
            Ok(())
        }
    }
}

impl Drop for CalibrationSender {
    fn drop(&mut self) {
        self.state.store(CalibrationState::Disconnected);
    }
}

pub struct CalibrationReceiver {
    state: Arc<AtomicCell<CalibrationState>>,
}

impl CalibrationReceiver {
    pub fn try_recv(&self) -> std::result::Result<Option<Input>, CalibrationDisconnected> {
        match self.state.load() {
            CalibrationState::Connected(input) => Ok(input),
            CalibrationState::Disconnected => Err(CalibrationDisconnected),
        }
    }
}

impl Drop for CalibrationReceiver {
    fn drop(&mut self) {
        self.state.store(CalibrationState::Disconnected);
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Config {
    pub driver: Driver,
    pub rumble: RumbleSetting,
    pub analog_scale: f64,
    #[cfg(target_os = "windows")]
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
            #[cfg(target_os = "windows")]
            vigem_config: Default::default(),
            calibration: Default::default(),
            ess: Default::default(),
        }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RumbleSetting {
    On,
    Off,
}

impl RumbleSetting {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::On, Self::Off]
    }
}

impl Default for RumbleSetting {
    fn default() -> Self {
        Self::On
    }
}
