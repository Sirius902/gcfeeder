use std::sync::Arc;

use async_trait::async_trait;
use gcinput::{Input, Rumble, STICK_RANGE};
use serde::{Deserialize, Serialize};
use tokio::pin;
use tokio::sync::Mutex;
use vigem_client as client;

use crate::util::packed_bools;

pub struct Driver {
    config: Config,
    device: Mutex<client::XTarget>,
    device_plugged: tokio::sync::Notify,
    tx_rumble: Arc<Mutex<tokio::sync::mpsc::Sender<Option<Rumble>>>>,
    rx_rumble: Mutex<tokio::sync::mpsc::Receiver<Option<Rumble>>>,
    notification_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Driver {
    pub fn new(config: Config, client: client::Client) -> Result<Self, client::Error> {
        let (tx_rumble, rx_rumble) = tokio::sync::mpsc::channel(1);

        Ok(Self {
            config,
            device: Mutex::new(match config.pad {
                Pad::Xbox360 => client::XTarget::new(client, client::TargetId::XBOX360_WIRED),
            }),
            device_plugged: tokio::sync::Notify::new(),
            tx_rumble: Arc::new(Mutex::new(tx_rumble)),
            rx_rumble: Mutex::new(rx_rumble),
            notification_task: Mutex::new(None),
        })
    }

    fn stick_coord_to_xinput(coord: u8) -> i16 {
        let scaled = f64::from(i16::from(coord) - i16::from(STICK_RANGE.center))
            / f64::from(STICK_RANGE.radius)
            * f64::from(i16::MAX);
        scaled.ceil() as i16
    }

    const fn apply_trigger_mode(trigger_mode: TriggerMode, input: &Input) -> TriggerResult {
        let l: u8;
        let r: u8;
        let mut ls: bool = false;
        let mut rs: bool = false;

        let Input {
            left_trigger,
            right_trigger,
            button_l,
            button_r,
            ..
        } = *input;

        match trigger_mode {
            TriggerMode::Analog => {
                l = left_trigger;
                r = right_trigger;
            }
            TriggerMode::Digital => {
                l = if button_l { u8::MAX } else { u8::MIN };
                r = if button_r { u8::MAX } else { u8::MIN };
            }
            TriggerMode::Combination => {
                l = if button_l { u8::MAX } else { left_trigger };
                r = if button_r { u8::MAX } else { right_trigger };
            }
            TriggerMode::StickClick => {
                l = left_trigger;
                r = right_trigger;
                ls = button_l;
                rs = button_r;
            }
        }

        TriggerResult { l, r, ls, rs }
    }

    fn input_to_xinput(config: &Config, input: &Input) -> client::XGamepad {
        let result = Self::apply_trigger_mode(config.trigger_mode, input);

        let buttons = packed_bools!((u16)
            input.button_up,
            input.button_down,
            input.button_left,
            input.button_right,
            input.button_start,
            false, // back
            result.ls, // left thumb
            result.rs, // right thumb
            false, // left shoulder
            input.button_z,
            false,
            false,
            input.button_a,
            input.button_b,
            input.button_x,
            input.button_y,
        );

        client::XGamepad {
            buttons: client::XButtons { raw: buttons },
            left_trigger: result.l,
            right_trigger: result.r,
            thumb_lx: Self::stick_coord_to_xinput(input.main_stick.x),
            thumb_ly: Self::stick_coord_to_xinput(input.main_stick.y),
            thumb_rx: Self::stick_coord_to_xinput(input.c_stick.x),
            thumb_ry: Self::stick_coord_to_xinput(input.c_stick.y),
        }
    }
}

#[async_trait]
impl super::Driver for Driver {
    fn name(&self) -> &'static str {
        "ViGEm"
    }

    async fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        let mut device = self.device.lock().await;
        let Some(input) = input else {
            if device.is_attached() {
                device.unplug()?;
            }

            return Ok(());
        };

        if !device.is_attached() {
            device.plugin()?;
            device.wait_ready()?;

            self.device_plugged.notify_one();
        }

        device.update(&Self::input_to_xinput(&self.config, input))?;

        Ok(())
    }

    async fn recv_rumble(&self) -> super::Result<Rumble> {
        loop {
            let mut notification_task = self.notification_task.lock().await;
            if notification_task.is_some() {
                let rumble = self
                    .rx_rumble
                    .lock()
                    .await
                    .recv()
                    .await
                    .expect("rumble channel is not closed");

                *notification_task = None;

                if let Some(rumble) = rumble {
                    return Ok(rumble);
                }
            }

            let mut device = self.device.lock().await;
            if !device.is_attached() {
                self.device_plugged.notified().await;
                continue;
            }

            if let Some(task) = notification_task.take() {
                task.await.expect("waiting for notification task");
            }

            let request_notification = device.request_notification()?;

            tracing::trace!("Launching notification task");
            notification_task.replace(tokio::task::spawn_blocking({
                let tx_rumble = self.tx_rumble.clone();
                move || {
                    pin!(request_notification);
                    request_notification.as_mut().request();
                    let notification = request_notification
                        .as_mut()
                        .as_mut()
                        .poll(true)
                        .ok()
                        .flatten();

                    if let Some(notification) = notification {
                        let rumble_strength =
                            notification.small_motor.max(notification.large_motor);

                        let _ = tx_rumble.blocking_lock().blocking_send(Some(
                            if rumble_strength == 0 {
                                Rumble::Off
                            } else {
                                Rumble::On
                            },
                        ));
                    } else {
                        let _ = tx_rumble.blocking_lock().blocking_send(None);
                    }
                }
            }));
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub pad: Pad,
    pub trigger_mode: TriggerMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pad: Pad::Xbox360,
            trigger_mode: TriggerMode::StickClick,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pad {
    Xbox360,
    // FUTURE(Sirius902) DualShock4 support in the ViGEm client library is not complete.
    // DualShock4,
}

impl Pad {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Xbox360]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    Analog,
    Digital,
    Combination,
    StickClick,
}

impl TriggerMode {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Analog,
            Self::Digital,
            Self::Combination,
            Self::StickClick,
        ]
    }
}

struct TriggerResult {
    l: u8,
    r: u8,
    ls: bool,
    rs: bool,
}
