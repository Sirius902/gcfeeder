use crate::{adapter, vjoy};
use adapter::Adapter;

#[derive(Debug)]
pub enum Error {
    Adapter(adapter::Error),
    VJoyEnable,
    VJoyDevice { r_id: u8, status: vjoy::VjdStat },
    VJoyAcquire { r_id: u8 },
}

pub struct Feeder {
    adapter: Adapter,
}

impl Feeder {
    pub fn new() -> Result<Feeder, Error> {
        Self::acquire_vjoy(1)?;

        Adapter::open()
            .map(|adapter| Feeder { adapter })
            .map_err(Error::Adapter)
    }

    pub fn feed(&mut self) -> Result<(), Error> {
        let result = self.adapter.read_inputs();

        if let Err(adapter::Error::Rusb(rusb::Error::Timeout)) = result {
            return Ok(());
        }

        let mut inputs = result.map_err(Error::Adapter)?;

        if let Some(input) = inputs[0].take() {
            let _ = unsafe { vjoy::UpdateVJD(1, &to_vjoy(input, 1)) };
        }

        Ok(())
    }

    fn acquire_vjoy(r_id: u8) -> Result<(), Error> {
        use vjoy::*;

        unsafe {
            if vJoyEnabled() != 0 {
                let status = GetVJDStatus(r_id.into());

                match status {
                    VjdStat::Own | VjdStat::Free => {
                        if AcquireVJD(1) == 0 {
                            return Err(Error::VJoyAcquire { r_id });
                        }
                    }
                    _ => return Err(Error::VJoyDevice { r_id, status }),
                }
            } else {
                return Err(Error::VJoyEnable);
            }
        }

        Ok(())
    }
}

impl Drop for Feeder {
    fn drop(&mut self) {
        unsafe {
            if let vjoy::VjdStat::Own = vjoy::GetVJDStatus(1) {
                let _ = vjoy::RelinquishVJD(1);
            }
        }
    }
}

pub fn to_vjoy(input: adapter::Input, b_device: u8) -> vjoy::JoystickPosition {
    const MULT: i32 = 0x7F;

    let mut pos = vjoy::JoystickPosition::zeroed();
    pos.b_device = b_device;

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
