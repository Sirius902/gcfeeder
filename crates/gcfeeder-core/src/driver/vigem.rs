use std::sync::{Arc, Mutex};
use std::{mem, thread};

use async_trait::async_trait;
use gcinput::{Input, Rumble, STICK_RANGE};
use serde::{Deserialize, Serialize};
use vigem_client as client;

use super::rumble::PatternRumbler;
use crate::util::packed_bools;

pub struct Driver {
    config: Config,
    device: Device,
}

impl Driver {
    pub fn new(config: Config, client: client::Client) -> Result<Self, client::Error> {
        match config.pad {
            Pad::Xbox360 => {
                let device = Device::new(client)?;
                Ok(Self { config, device })
            }
        }
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
        tokio::task::spawn_blocking({
            let input = *input;
            let config = self.config;
            let target = self.device.target.clone();
            let thread = self.device.notification_thread.clone();
            let rumbler = self.device.rumbler.clone();
            move || {
                let mut target = target.lock().unwrap();
                let mut thread = thread.lock().unwrap();

                let target = target.as_mut().unwrap();

                if let Some(input) = input {
                    if !target.is_attached() {
                        Device::plugin_locked(target, &mut thread, rumbler.clone())?;
                    }

                    target.update(&Self::input_to_xinput(&config, &input))?;
                } else if target.is_attached() {
                    target.unplug()?;
                }

                Ok(())
            }
        })
        .await
        .expect("feed")
    }

    async fn peek_rumble_state(&self) -> Rumble {
        tokio::task::spawn_blocking({
            let rumbler = self.device.rumbler.clone();
            move || match rumbler.lock().unwrap().peek_rumble() {
                true => Rumble::On,
                false => Rumble::Off,
            }
        })
        .await
        .expect("peek rumble state")
    }

    async fn consume_rumble_state(&self) -> Rumble {
        tokio::task::spawn_blocking({
            let rumbler = self.device.rumbler.clone();
            move || match rumbler.lock().unwrap().consume_rumble() {
                true => Rumble::On,
                false => Rumble::Off,
            }
        })
        .await
        .expect("consume rumble state")
    }
}

#[derive(Debug)]
struct Device {
    target: Arc<Mutex<Option<client::XTarget>>>,
    notification_thread: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    rumbler: Arc<Mutex<PatternRumbler>>,
}

impl Device {
    pub fn new(client: client::Client) -> Result<Self, client::Error> {
        let target = client::XTarget::new(client, vigem_client::TargetId::XBOX360_WIRED);

        Ok(Self {
            target: Arc::new(Mutex::new(Some(target))),
            notification_thread: Arc::new(Mutex::new(None)),
            rumbler: Arc::new(Mutex::new(PatternRumbler::new())),
        })
    }

    pub fn plugin_locked(
        target: &mut client::XTarget,
        notification_thread: &mut Option<thread::JoinHandle<()>>,
        rumbler: Arc<Mutex<PatternRumbler>>,
    ) -> super::Result<()> {
        let thread = target
            .plugin()
            .and_then(|()| target.wait_ready())
            .and_then(|()| target.request_notification())
            .map(|notification| {
                notification.spawn_thread(move |_, data| {
                    rumbler
                        .lock()
                        .unwrap()
                        .update_strength(data.small_motor.max(data.large_motor));
                })
            });

        match thread {
            Ok(thread) => {
                *notification_thread = Some(thread);
                Ok(())
            }
            Err(e) => {
                if target.is_attached() {
                    let _ = target.unplug();
                }

                Err(e.into())
            }
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        *self.target.lock().unwrap() = None;

        if let Some(thread) = self.notification_thread.lock().unwrap().take() {
            mem::drop(thread.join());
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
