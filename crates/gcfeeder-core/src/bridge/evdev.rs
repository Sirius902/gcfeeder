use std::{
    io, mem,
    os::fd::{AsRawFd, BorrowedFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use enclose::enclose;
use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, FFEffectCode,
    InputEvent, KeyCode, UinputAbsSetup,
};
use gcinput::{Input, Rumble, STICK_RANGE, TRIGGER_RANGE};
use nix::{
    fcntl::{FcntlArg, OFlag},
    sys::epoll,
};

use super::{rumble::PatternRumbler, Bridge};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("unix: {0}")]
    Unix(#[from] nix::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct EvdevBridge {
    device: Arc<Mutex<Option<VirtualDevice>>>,
    epoll_handle: Arc<Mutex<Option<epoll::Epoll>>>,
    rumbler: Arc<Mutex<PatternRumbler>>,
    stop_flag: Arc<AtomicBool>,
    rumble_thread: Option<thread::JoinHandle<()>>,
}

impl EvdevBridge {
    pub fn new() -> Self {
        let device = Arc::new(Mutex::new(None));
        let epoll_handle = Arc::new(Mutex::new(None));
        let rumbler = Arc::new(Mutex::new(Default::default()));
        let stop_flag = Arc::new(AtomicBool::new(false));

        let rumble_thread = Some(thread::spawn(
            enclose!((device, epoll_handle, rumbler, stop_flag) move || Self::rumble_loop(device, epoll_handle, rumbler, stop_flag)),
        ));

        Self {
            device,
            epoll_handle,
            rumbler,
            stop_flag,
            rumble_thread,
        }
    }

    fn create_device() -> Result<VirtualDevice> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_SOUTH); // A
        keys.insert(KeyCode::BTN_EAST); // B
        keys.insert(KeyCode::BTN_WEST); // X
        keys.insert(KeyCode::BTN_NORTH); // Y
        keys.insert(KeyCode::BTN_START); // Start
        keys.insert(KeyCode::BTN_TR); // Z
        keys.insert(KeyCode::BTN_THUMBL); // L
        keys.insert(KeyCode::BTN_THUMBR); // R

        let stick_axis_info = AbsInfo::new(
            STICK_RANGE.center.into(),
            (STICK_RANGE.center - STICK_RANGE.radius).into(),
            (STICK_RANGE.center + STICK_RANGE.radius).into(),
            0,
            0,
            // TODO: Find out if this is a reasonable value.
            50,
        );

        let trigger_axis_info = AbsInfo::new(
            TRIGGER_RANGE.min.into(),
            TRIGGER_RANGE.min.into(),
            TRIGGER_RANGE.max.into(),
            0,
            0,
            // TODO: Find out if this is a reasonable value.
            50,
        );

        let hat_axis_info = AbsInfo::new(0, -1, 1, 0, 0, 50);

        Ok(VirtualDevice::builder()?
            .name("gcfeeder | GameCube Controller")
            .with_ff(&AttributeSet::from_iter([FFEffectCode::FF_RUMBLE]))?
            .with_ff_effects_max(1)
            .with_keys(&keys)?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_X,
                stick_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Y,
                stick_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RX,
                stick_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RY,
                stick_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_Z,
                trigger_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_RZ,
                trigger_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_HAT0X,
                hat_axis_info,
            ))?
            .with_absolute_axis(&UinputAbsSetup::new(
                AbsoluteAxisCode::ABS_HAT0Y,
                hat_axis_info,
            ))?
            .build()?)
    }

    // TODO: Use tokio instead of epoll?
    fn create_epoll(device: &VirtualDevice) -> Result<epoll::Epoll> {
        let raw_fd = device.as_raw_fd();
        nix::fcntl::fcntl(raw_fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;

        let event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, 0);
        let epoll_handle = epoll::Epoll::new(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        // Safety: Epoll must be dropped before VirtualDevice is dropped.
        epoll_handle.add(unsafe { BorrowedFd::borrow_raw(raw_fd) }, event)?;
        Ok(epoll_handle)
    }

    fn rumble_loop(
        device: Arc<Mutex<Option<VirtualDevice>>>,
        epoll_handle: Arc<Mutex<Option<epoll::Epoll>>>,
        rumbler: Arc<Mutex<PatternRumbler>>,
        stop_flag: Arc<AtomicBool>,
    ) {
        while !stop_flag.load(Ordering::Acquire) {
            let mut epoll_handle_opt = epoll_handle.lock().unwrap();
            let Some(epoll_handle) = &mut *epoll_handle_opt else {
                std::mem::drop(epoll_handle_opt);
                thread::sleep(Duration::from_millis(8));
                continue;
            };

            let events = device
                .lock()
                .unwrap()
                .as_mut()
                .map(VirtualDevice::fetch_events)
                .map(|opt| opt.map(|it| it.collect::<Vec<InputEvent>>()));

            let Some(events) = events else {
                std::mem::drop(epoll_handle_opt);
                thread::sleep(Duration::from_millis(8));
                continue;
            };

            match events {
                Ok(events) => {
                    let mut device_opt = device.lock().unwrap();
                    let Some(device) = &mut *device_opt else {
                        std::mem::drop(epoll_handle_opt);
                        std::mem::drop(device_opt);
                        thread::sleep(Duration::from_millis(8));
                        continue;
                    };

                    for event in events.into_iter() {
                        match event.destructure() {
                            evdev::EventSummary::UInput(event, code, _value)
                                if code == evdev::UInputCode::UI_FF_UPLOAD =>
                            {
                                let Ok(mut event) = device.process_ff_upload(event) else {
                                    thread::sleep(Duration::from_millis(8));
                                    continue;
                                };

                                match event.effect().kind {
                                    evdev::FFEffectKind::Rumble {
                                        strong_magnitude,
                                        weak_magnitude,
                                    } => {
                                        let strength = ((strong_magnitude.max(weak_magnitude)
                                            as f32)
                                            / (u16::MAX as f32)
                                            * (u8::MAX as f32))
                                            .round()
                                            as u8;

                                        rumbler.lock().unwrap().update_strength(strength);
                                    }
                                    _ => {
                                        log::warn!(
                                            "unsupported ff effect: {:?}",
                                            event.effect().kind
                                        );
                                    }
                                }

                                event.set_effect_id(0);
                                event.set_retval(0);
                            }
                            evdev::EventSummary::UInput(event, code, _value)
                                if code == evdev::UInputCode::UI_FF_ERASE =>
                            {
                                let Ok(event) = device.process_ff_erase(event) else {
                                    thread::sleep(Duration::from_millis(8));
                                    continue;
                                };

                                if event.effect_id() == 0 {
                                    let mut rumbler = rumbler.lock().unwrap();
                                    rumbler.update_strength(0);
                                }
                            }
                            evdev::EventSummary::ForceFeedback(_ev, _code, _value) => {}
                            _ => {
                                log::debug!("Unknown evdev event = {:?}", event);
                            }
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    let mut events = [epoll::EpollEvent::empty(); 2];
                    let _ = epoll_handle.wait(
                        &mut events,
                        epoll::EpollTimeout::try_from(Duration::from_millis(8)).unwrap(),
                    );
                    continue;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(8));
                    continue;
                }
            };
        }
    }
}

impl Default for EvdevBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EvdevBridge {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Release);

        if let Some(t) = self.rumble_thread.take() {
            mem::drop(t.join());
        }
    }
}

impl Bridge for EvdevBridge {
    fn driver_name(&self) -> &'static str {
        "evdev"
    }

    fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        let Some(input) = input else {
            *self.epoll_handle.lock().unwrap() = None;
            *self.device.lock().unwrap() = None;
            *self.rumbler.lock().unwrap() = Default::default();
            return Ok(());
        };

        let mut device_opt = self.device.lock().unwrap();
        let device = match &mut *device_opt {
            Some(d) => d,
            None => {
                let device = device_opt.insert(Self::create_device()?);
                *self.epoll_handle.lock().unwrap() = Some(Self::create_epoll(device)?);
                device
            }
        };

        let btn_state = |b: bool| {
            if b {
                1
            } else {
                0
            }
        };

        let hat_state = |pos: bool, neg: bool| match (pos, neg) {
            (true, false) => 1,
            (false, true) => -1,
            _ => 0,
        };

        // TODO: Create a report based on the diff from the last input.
        device
            .emit(&[
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_SOUTH.0,
                    btn_state(input.button_a),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_EAST.0,
                    btn_state(input.button_b),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_WEST.0,
                    btn_state(input.button_x),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_NORTH.0,
                    btn_state(input.button_y),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_START.0,
                    btn_state(input.button_start),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_TR.0,
                    btn_state(input.button_z),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_THUMBL.0,
                    btn_state(input.button_l),
                ),
                InputEvent::new(
                    EventType::KEY.0,
                    KeyCode::BTN_THUMBR.0,
                    btn_state(input.button_r),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_X.0,
                    input.main_stick.x.into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_Y.0,
                    (!input.main_stick.y).into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_RX.0,
                    input.c_stick.x.into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_RY.0,
                    (!input.c_stick.y).into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_Z.0,
                    input.left_trigger.into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_RZ.0,
                    input.right_trigger.into(),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_HAT0X.0,
                    hat_state(input.button_right, input.button_left),
                ),
                InputEvent::new(
                    EventType::ABSOLUTE.0,
                    AbsoluteAxisCode::ABS_HAT0Y.0,
                    hat_state(input.button_down, input.button_up),
                ),
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
