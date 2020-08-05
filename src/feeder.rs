use crate::{adapter, vjoy};
use adapter::{Adapter, Rumble};

const DEVICE_ID: u32 = 1;

#[derive(Debug)]
pub enum Error {
    Adapter(adapter::Error),
    VJoy(vjoy::Error),
}

pub struct Feeder {
    adapter: Adapter,
    device: vjoy::Device,
    rumble_enabled: bool,
    previous_rumble: Rumble,
}

impl Feeder {
    pub fn new() -> Result<Feeder, Error> {
        let adapter = Adapter::open().map_err(Error::Adapter)?;
        let device = vjoy::Device::acquire(DEVICE_ID).map_err(Error::VJoy)?;
        let rumble_enabled = device.is_ffb();

        if rumble_enabled {
            vjoy::start_ffb();
        }

        Ok(Feeder {
            adapter,
            device,
            rumble_enabled,
            previous_rumble: Rumble::Off,
        })
    }

    pub fn feed(&mut self) -> Result<(), Error> {
        let result = self.adapter.read_inputs();

        if let Err(adapter::Error::Rusb(rusb::Error::Timeout)) = result {
            return Ok(());
        }

        let mut inputs = result.map_err(Error::Adapter)?;

        if let Some(input) = inputs[0].take() {
            let _ = self.device.update(input.into());
        }

        if self.rumble_enabled {
            self.update_rumble()?;
        }

        Ok(())
    }

    fn update_rumble(&mut self) -> Result<(), Error> {
        let ffb_status = vjoy::FFB_STATUS
            .read()
            .unwrap()
            .get(&DEVICE_ID)
            .cloned()
            .unwrap_or(vjoy::FFBOp::Stop);

        let rumble = Rumble::from(ffb_status);

        if self.previous_rumble != rumble {
            self.previous_rumble = rumble;

            let result = self
                .adapter
                .set_rumble([rumble, Rumble::Off, Rumble::Off, Rumble::Off]);

            if let Err(adapter::Error::Rusb(rusb::Error::Timeout)) = result {
                return Ok(());
            } else {
                return result.map_err(Error::Adapter);
            }
        }

        Ok(())
    }
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

// TODO: Not sure what `Solo` is. Consider ignoring it for rumble purposes?
impl From<vjoy::FFBOp> for Rumble {
    fn from(operation: vjoy::FFBOp) -> Rumble {
        use vjoy::FFBOp::*;

        match operation {
            Start | Solo => Rumble::On,
            Stop => Rumble::Off,
        }
    }
}
