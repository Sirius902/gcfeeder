use crate::{adapter, vjoy};
use adapter::{
    rumble::{Rumble, Rumbler},
    Adapter,
};
use crossbeam::channel;
use std::thread;
use std::time::{Duration, Instant};
use stoppable_thread::StoppableHandle;

const DEVICE_ID: u32 = 1;

#[derive(Debug)]
pub enum Error {
    Adapter(adapter::Error),
    VJoy(vjoy::Error),
}

pub struct Feeder {
    input_thread: Option<StoppableHandle<()>>,
    rumble_thread: Option<StoppableHandle<()>>,
    pub error_receiver: channel::Receiver<Error>,
}

impl Feeder {
    pub fn new() -> Result<Feeder, Error> {
        let adapter = Adapter::open().map_err(Error::Adapter)?;
        let device = vjoy::Device::acquire(DEVICE_ID).map_err(Error::VJoy)?;
        let rumbler = if device.is_ffb() {
            Some(adapter.make_rumbler())
        } else {
            None
        };

        let (error_sender, error_receiver) = channel::unbounded();

        let input_thread = Some(spawn_input_thread(adapter, device, error_sender.clone()));
        let rumble_thread =
            rumbler.map(|rumbler| spawn_rumble_thread(rumbler, vjoy::receive_ffb(), error_sender));

        Ok(Feeder {
            input_thread,
            rumble_thread,
            error_receiver,
        })
    }
}

impl Drop for Feeder {
    fn drop(&mut self) {
        if let Some(input_thread) = self.input_thread.take() {
            input_thread.stop().join().unwrap();
        }

        if let Some(rumble_thread) = self.rumble_thread.take() {
            rumble_thread.stop().join().unwrap();
        }
    }
}

fn spawn_input_thread(
    mut adapter: Adapter,
    device: vjoy::Device,
    error_sender: channel::Sender<Error>,
) -> StoppableHandle<()> {
    stoppable_thread::spawn(move |stopped| loop {
        if stopped.get() {
            break;
        }

        let result = adapter.read_inputs();

        match result {
            Err(adapter::Error::Rusb(rusb::Error::Timeout)) => {}
            Err(err) => {
                error_sender.send(Error::Adapter(err)).unwrap();
            }
            Ok(mut inputs) => {
                if let Some(input) = inputs[0].take() {
                    let _ = device.update(input.into());
                }
            }
        }

        thread::yield_now();
    })
}

fn spawn_rumble_thread(
    rumbler: Rumbler,
    ffb_reciever: channel::Receiver<vjoy::FFBPacket>,
    error_sender: channel::Sender<Error>,
) -> StoppableHandle<()> {
    let mut rumble_duration = Duration::from_secs(1);
    let mut rumble_timer = Duration::default();
    let mut then = Instant::now();
    let mut last_effect: Option<Instant> = None;

    stoppable_thread::spawn(move |stopped| loop {
        if stopped.get() {
            break;
        }

        match ffb_reciever.try_recv() {
            Ok(vjoy::FFBPacket::EffectOperation {
                device_id,
                operation,
                timestamp,
            }) => {
                if device_id == DEVICE_ID {
                    /* Kind of a janky hack, but stop rumbling the controller if two effect
                     * operation packets arrive less than a millisecond from each other since
                     * they may be out of order. */
                    let quick_packet = last_effect
                        .map(|last_effect| (timestamp - last_effect).as_millis() < 1)
                        .unwrap_or(false);

                    if operation.effect_op == vjoy::FFBOp::Stop || quick_packet {
                        rumble_timer = Duration::default();
                    } else {
                        rumble_timer = rumble_duration * operation.loop_count.into();
                    }

                    last_effect = Some(timestamp);
                }
            }
            Ok(vjoy::FFBPacket::EffectReport { device_id, report }) => {
                if device_id == DEVICE_ID {
                    rumble_duration = Duration::from_millis(report.duration.into());
                }
            }
            Err(_) => {}
        }

        let rumble = if rumble_timer.as_millis() > 0 {
            Rumble::On
        } else {
            Rumble::Off
        };

        let result = rumbler.set_rumble([rumble, Rumble::Off, Rumble::Off, Rumble::Off]);

        match result {
            Err(adapter::Error::Rusb(rusb::Error::Timeout)) => {}
            Err(err) => {
                error_sender.send(Error::Adapter(err)).unwrap();
            }
            _ => {}
        }

        rumble_timer = rumble_timer.checked_sub(then.elapsed()).unwrap_or_default();
        then = Instant::now();

        thread::yield_now();
    })
}

impl From<adapter::Input> for vjoy::JoystickPosition {
    fn from(input: adapter::Input) -> vjoy::JoystickPosition {
        const MULT: i32 = 0x7F;

        let mut pos = vjoy::JoystickPosition::zeroed();

        pos.l_buttons = input.button_a as i32
            | (input.button_b as i32) << 1
            | (input.button_x as i32) << 2
            | (input.button_y as i32) << 3
            | (input.button_z as i32) << 4
            | (input.button_r as i32) << 5
            | (input.button_l as i32) << 6
            | (input.button_start as i32) << 7
            | (input.button_up as i32) << 8
            | (input.button_down as i32) << 9
            | (input.button_left as i32) << 10
            | (input.button_right as i32) << 11;

        pos.w_axis_x = i32::from(input.stick_x) * MULT;
        pos.w_axis_y = i32::from(!input.stick_y) * MULT;

        pos.w_axis_x_rot = i32::from(input.substick_x) * MULT;
        pos.w_axis_y_rot = i32::from(!input.substick_y) * MULT;

        pos.w_axis_z = i32::from(input.trigger_left) * MULT;
        pos.w_axis_z_rot = i32::from(input.trigger_right) * MULT;

        pos
    }
}
