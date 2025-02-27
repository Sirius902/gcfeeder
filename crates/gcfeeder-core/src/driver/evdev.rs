use async_trait::async_trait;
use evdev::uinput::{VirtualDevice, VirtualEventStream};
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, FFEffectCode, InputEvent, KeyCode,
    UinputAbsSetup,
};
use gcinput::{Input, Rumble, STICK_RANGE, TRIGGER_RANGE};
use tokio::sync::Mutex;
use tracing::debug;

use super::rumble::PatternRumbler;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct Driver {
    stream: Mutex<Option<VirtualEventStream>>,
    stream_created: tokio::sync::Notify,
}

impl Driver {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_stream() -> Result<VirtualEventStream> {
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
            // FUTURE(Sirius902) Find out if this is a reasonable value.
            50,
        );

        let trigger_axis_info = AbsInfo::new(
            TRIGGER_RANGE.min.into(),
            TRIGGER_RANGE.min.into(),
            TRIGGER_RANGE.max.into(),
            0,
            0,
            // FUTURE(Sirius902) Find out if this is a reasonable value.
            50,
        );

        let hat_axis_info = AbsInfo::new(
            0, -1, 1, 0, 0, // FUTURE(Sirius902) Find out if this is a reasonable value.
            50,
        );

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
            .build()?
            .into_event_stream()?)
    }

    fn write_inputs(device: &mut VirtualDevice, input: &Input) -> std::io::Result<()> {
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

        // FUTURE(Sirius902) Create a report based on the diff from the last input.
        device.emit(&[
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
        ])?;

        Ok(())
    }

    async fn handle_rumble_event(
        device: &mut VirtualDevice,
        rumbler: &mut PatternRumbler,
        event: &evdev::InputEvent,
    ) -> Result<()> {
        match event.destructure() {
            // TODO(Sirius902) Properly handle FF, setting rumble strength should happen in
            // `evdev::EventSummary::ForceFeedback`.
            evdev::EventSummary::UInput(event, evdev::UInputCode::UI_FF_UPLOAD, _value) => {
                let mut event = device.process_ff_upload(event)?;

                match event.effect().kind {
                    evdev::FFEffectKind::Rumble {
                        strong_magnitude,
                        weak_magnitude,
                    } => {
                        let strength = ((strong_magnitude.max(weak_magnitude) as f32)
                            / (u16::MAX as f32)
                            * (u8::MAX as f32))
                            .round() as u8;

                        rumbler.update_strength(strength);
                    }
                    _ => {
                        debug!("Unsupported ff effect: {:?}", event.effect().kind);
                    }
                }

                event.set_effect_id(0);
                event.set_retval(0);
            }
            evdev::EventSummary::UInput(event, evdev::UInputCode::UI_FF_ERASE, _value) => {
                let event = device.process_ff_erase(event)?;

                if event.effect_id() == 0 {
                    rumbler.update_strength(0);
                }
            }
            evdev::EventSummary::ForceFeedback(_ev, _code, _value) => {}
            _ => {
                debug!("Unknown evdev event = {:?}", event);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl super::Driver for Driver {
    fn name(&self) -> &'static str {
        "evdev"
    }

    async fn feed(&self, input: &Option<Input>) -> super::Result<()> {
        let mut stream = self.stream.lock().await;
        let Some(input) = input else {
            if let Some(stream) = stream.take() {
                // TODO(Sirius902) I don't think I should have to do this but trying to drop
                // `stream` in the current task will block it indefinitely. Ship `stream` off to
                // be dropped in a separate task.
                tokio::task::spawn_blocking(move || {
                    drop(stream);
                });
            }

            return Ok(());
        };

        let stream = {
            if let Some(stream) = stream.as_mut() {
                stream
            } else {
                let stream = stream.insert(Self::create_stream()?);
                self.stream_created.notify_one();
                stream
            }
        };

        Self::write_inputs(stream.device_mut(), input).map_err(Error::Io)?;

        Ok(())
    }

    async fn recv_rumble(&self) -> super::Result<Rumble> {
        loop {
            let mut stream = self.stream.lock().await;
            let Some(stream) = stream.as_mut() else {
                drop(stream);
                self.stream_created.notified().await;
                continue;
            };

            let event = stream.next_event().await.map_err(Error::Io)?;
            let device = stream.device_mut();

            match event.destructure() {
                // TODO(Sirius902) Properly handle FF, setting rumble strength should happen in
                // `evdev::EventSummary::ForceFeedback`.
                evdev::EventSummary::UInput(event, evdev::UInputCode::UI_FF_UPLOAD, _value) => {
                    let mut event = device.process_ff_upload(event).map_err(Error::Io)?;

                    match event.effect().kind {
                        evdev::FFEffectKind::Rumble {
                            strong_magnitude,
                            weak_magnitude,
                        } => {
                            let strength = ((strong_magnitude.max(weak_magnitude) as f32)
                                / (u16::MAX as f32)
                                * (u8::MAX as f32))
                                .round() as u8;

                            return Ok(match strength {
                                0 => Rumble::Off,
                                _ => Rumble::On,
                            });
                        }
                        _ => {
                            debug!("Unsupported ff effect: {:?}", event.effect().kind);
                        }
                    }

                    event.set_effect_id(0);
                    event.set_retval(0);
                }
                evdev::EventSummary::UInput(event, evdev::UInputCode::UI_FF_ERASE, _value) => {
                    let event = device.process_ff_erase(event).map_err(Error::Io)?;

                    if event.effect_id() == 0 {
                        return Ok(Rumble::Off);
                    }
                }
                evdev::EventSummary::ForceFeedback(_ev, _code, _value) => {}
                _ => {
                    debug!("Unknown evdev event = {:?}", event);
                }
            }
        }
    }
}
