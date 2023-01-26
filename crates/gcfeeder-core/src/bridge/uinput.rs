use std::{fs, io, sync::Mutex, time};

use gcinput::{Input, Rumble, STICK_RANGE, TRIGGER_RANGE};
use input_linux::{sys, InputId, UInputHandle};

use super::{rumble::PatternRumbler, Bridge};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

type Device = UInputHandle<fs::File>;

#[derive(Default)]
pub struct UInputBridge {
    device: Mutex<Option<Device>>,
    rumbler: Mutex<PatternRumbler>,
}

impl UInputBridge {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_device() -> Result<Device> {
        let device = UInputHandle::new(fs::File::create("/dev/uinput")?);

        let input_id = InputId {
            bustype: sys::BUS_VIRTUAL,
            vendor: 0x7331,
            product: 0x0069,
            version: 1,
        };

        device.set_evbit(input_linux::EventKind::Absolute)?;
        device.set_evbit(input_linux::EventKind::Key)?;

        device.set_keybit(input_linux::Key::Button0)?; // A
        device.set_keybit(input_linux::Key::Button1)?; // B
        device.set_keybit(input_linux::Key::Button2)?; // X
        device.set_keybit(input_linux::Key::Button3)?; // Y
        device.set_keybit(input_linux::Key::Button4)?; // Start
        device.set_keybit(input_linux::Key::Button5)?; // Z
        device.set_keybit(input_linux::Key::Button6)?; // L
        device.set_keybit(input_linux::Key::Button7)?; // R

        let stick_info = input_linux::AbsoluteInfo {
            value: STICK_RANGE.center.into(),
            minimum: (STICK_RANGE.center - STICK_RANGE.radius).into(),
            maximum: (STICK_RANGE.center + STICK_RANGE.radius).into(),
            fuzz: 0,
            flat: 0,
            // TODO: Find out if this is a reasonable value.
            resolution: 50,
        };

        let trigger_info = input_linux::AbsoluteInfo {
            value: TRIGGER_RANGE.min.into(),
            minimum: TRIGGER_RANGE.min.into(),
            maximum: TRIGGER_RANGE.max.into(),
            fuzz: 0,
            flat: 0,
            // TODO: Find out if this is a reasonable value.
            resolution: 50,
        };

        let hat_info = input_linux::AbsoluteInfo {
            value: 0,
            minimum: -1,
            maximum: 1,
            fuzz: 0,
            flat: 0,
            // TODO: Find out if this is a reasonable value.
            resolution: 50,
        };

        device.create(
            &input_id,
            b"gcfeeder | GameCube Controller",
            1,
            &[
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::X,
                    info: stick_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::Y,
                    info: stick_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::RX,
                    info: stick_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::RY,
                    info: stick_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::Z,
                    info: trigger_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::RZ,
                    info: trigger_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::Hat0X,
                    info: hat_info,
                },
                input_linux::AbsoluteInfoSetup {
                    axis: input_linux::AbsoluteAxis::Hat0Y,
                    info: hat_info,
                },
            ],
        )?;

        Ok(device)
    }
}

impl Bridge for UInputBridge {
    fn driver_name(&self) -> &'static str {
        "uinput"
    }

    fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        let Some(input) = input else {
            *self.device.lock().unwrap() = None;
            return Ok(());
        };

        let mut device_opt = self.device.lock().unwrap();
        let device = match &*device_opt {
            Some(d) => d,
            None => device_opt.insert(Self::create_device()?),
        };

        let btn_state = |b: bool| {
            if b {
                input_linux::KeyState::PRESSED
            } else {
                input_linux::KeyState::RELEASED
            }
        };

        let now = time::SystemTime::now();
        let unix_now = now.duration_since(time::SystemTime::UNIX_EPOCH).unwrap();
        let secs = i64::try_from(unix_now.as_secs()).unwrap();
        let usecs = i64::from(unix_now.subsec_micros());
        let event_time = input_linux::EventTime::new(secs, usecs);

        let _ = device
            .write(&[
                *input_linux::KeyEvent::new(
                    event_time,
                    input_linux::Key::Button0,
                    btn_state(input.button_a),
                )
                .as_ref(),
                *input_linux::AbsoluteEvent::new(
                    event_time,
                    input_linux::AbsoluteAxis::X,
                    input.main_stick.x.into(),
                )
                .as_ref(),
                *input_linux::SynchronizeEvent::report(event_time).as_ref(),
            ])
            .map_err(Error::Io)?;

        Ok(())
    }

    fn rumble_state(&self) -> Rumble {
        if self.rumbler.lock().unwrap().peek_rumble() {
            Rumble::On
        } else {
            Rumble::Off
        }
    }

    fn notify_rumble_consumed(&self) {
        let _ = self.rumbler.lock().unwrap().poll_rumble();
    }
}
