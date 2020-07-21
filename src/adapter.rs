use rusb::constants::LIBUSB_DT_HID;
use rusb::{DeviceHandle, Direction, GlobalContext};
use std::time::Duration;

pub const MAIN_STICK_CENTER_X: u8 = 0x80;
pub const MAIN_STICK_CENTER_Y: u8 = 0x80;
pub const MAIN_STICK_RADIUS: u8 = 0x7F;
pub const C_STICK_CENTER_X: u8 = 0x80;
pub const C_STICK_CENTER_Y: u8 = 0x80;
pub const C_STICK_RADIUS: u8 = 0x7F;

const PAYLOAD_SIZE: usize = 37;
const ALLOWED_TIMEOUT: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub enum Error {
    Rusb(rusb::Error),
    Adapter,
    Payload,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Port {
    One,
    Two,
    Three,
    Four,
}

impl Port {
    pub fn channel(self) -> usize {
        match self {
            Port::One => 0,
            Port::Two => 1,
            Port::Three => 2,
            Port::Four => 3,
        }
    }

    pub fn all() -> &'static [Port] {
        const PORTS: [Port; 4] = [Port::One, Port::Two, Port::Three, Port::Four];
        &PORTS
    }
}

pub struct Adapter {
    handle: DeviceHandle<GlobalContext>,
    endpoint_in: u8,
    /// `None` if the controller on the channel is unplugged. If it is plugged,
    /// the drift is calculated once when connected on `read_inputs`.
    drifts: [Option<Drift>; 4],
}

impl Adapter {
    pub fn open() -> Result<Adapter, Error> {
        const GC_VID: u16 = 0x057E;
        const GC_PID: u16 = 0x0337;

        let mut handle = rusb::open_device_with_vid_pid(GC_VID, GC_PID).ok_or(Error::Adapter)?;

        handle.claim_interface(0).map_err(Error::Rusb)?;

        let (endpoint_in, endpoint_out) = Self::endpoints(&handle)?;

        // Not sure what this does but Dolphin does it
        let _ = handle
            .write_interrupt(endpoint_out, &[0x13], ALLOWED_TIMEOUT)
            .map_err(Error::Rusb)?;

        Ok(Adapter {
            handle,
            endpoint_in,
            drifts: Default::default(),
        })
    }

    pub fn close(&mut self) -> Result<(), Error> {
        self.handle.release_interface(0).map_err(Error::Rusb)
    }

    pub fn read_inputs(&mut self) -> Result<[Option<Input>; 4], Error> {
        let payload = self.read_payload()?;
        let mut inputs: [Option<Input>; 4] = Default::default();

        for port in Port::all() {
            let chan = port.channel();
            // type is 0 if no controller is plugged, 1 if wired, and 2 if wireless
            let r#type = payload[1 + (9 * chan)] >> 4;
            let connected = r#type != 0;

            if !connected {
                self.drifts[chan] = None;
                continue;
            }

            let raw = Input::parse(&payload, *port);
            let drift = self.drifts[chan].get_or_insert_with(|| Drift::new(&raw));

            inputs[chan] = Some(drift.correct(raw));
        }

        Ok(inputs)
    }

    fn read_payload(&self) -> Result<[u8; PAYLOAD_SIZE], Error> {
        let mut payload = [0; PAYLOAD_SIZE];

        let bytes_read = self
            .handle
            .read_interrupt(self.endpoint_in, &mut payload, ALLOWED_TIMEOUT)
            .map_err(Error::Rusb)?;

        if bytes_read != PAYLOAD_SIZE || payload[0] != LIBUSB_DT_HID {
            return Err(Error::Payload);
        }

        Ok(payload)
    }

    /// Returns the in and out endpoints for the adapter.
    ///
    /// The in endpoint is for reading controller inputs while the out endpoint
    /// is for writing rumble data.
    fn endpoints(handle: &DeviceHandle<GlobalContext>) -> Result<(u8, u8), Error> {
        let mut endpoint_in = 0;
        let mut endpoint_out = 0;
        let device = handle.device();
        let config = device.config_descriptor(0).map_err(Error::Rusb)?;

        for interface_container in config.interfaces() {
            for interface in interface_container.descriptors() {
                for endpoint in interface.endpoint_descriptors() {
                    match endpoint.direction() {
                        Direction::In => {
                            endpoint_in = endpoint.address();
                        }
                        Direction::Out => {
                            endpoint_out = endpoint.address();
                        }
                    }
                }
            }
        }

        Ok((endpoint_in, endpoint_out))
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[derive(Debug)]
pub struct Drift {
    stick_x: i16,
    stick_y: i16,
    substick_x: i16,
    substick_y: i16,
}

impl Drift {
    pub fn new(initial: &Input) -> Drift {
        Drift {
            stick_x: i16::from(MAIN_STICK_CENTER_X) - i16::from(initial.stick_x),
            stick_y: i16::from(MAIN_STICK_CENTER_Y) - i16::from(initial.stick_y),
            substick_x: i16::from(C_STICK_CENTER_X) - i16::from(initial.substick_x),
            substick_y: i16::from(C_STICK_CENTER_Y) - i16::from(initial.substick_y),
        }
    }

    pub fn correct(&self, mut input: Input) -> Input {
        let constrict =
            |n: i16, center: i16, radius: i16| n.min(center + radius).max(center - radius);

        input.stick_x = constrict(
            i16::from(input.stick_x) + self.stick_x,
            i16::from(MAIN_STICK_CENTER_X),
            i16::from(MAIN_STICK_RADIUS),
        ) as u8;
        input.stick_y = constrict(
            i16::from(input.stick_y) + self.stick_y,
            i16::from(MAIN_STICK_CENTER_Y),
            i16::from(MAIN_STICK_RADIUS),
        ) as u8;
        input.substick_x = constrict(
            i16::from(input.substick_x) + self.substick_x,
            i16::from(C_STICK_CENTER_X),
            i16::from(C_STICK_RADIUS),
        ) as u8;
        input.substick_y = constrict(
            i16::from(input.substick_y) + self.substick_y,
            i16::from(C_STICK_CENTER_Y),
            i16::from(C_STICK_RADIUS),
        ) as u8;

        input
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Input {
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,

    pub button_left: bool,
    pub button_right: bool,
    pub button_down: bool,
    pub button_up: bool,

    pub button_start: bool,
    pub button_z: bool,
    pub button_r: bool,
    pub button_l: bool,

    pub stick_x: u8,
    pub stick_y: u8,
    pub substick_x: u8,
    pub substick_y: u8,
    pub trigger_left: u8,
    pub trigger_right: u8,
}

impl Input {
    fn parse(payload: &[u8; PAYLOAD_SIZE], port: Port) -> Input {
        let chan = port.channel();
        let b1 = payload[1 + (9 * chan) + 1];
        let b2 = payload[1 + (9 * chan) + 2];

        Input {
            button_a: (b1 & (1 << 0)) != 0,
            button_b: (b1 & (1 << 1)) != 0,
            button_x: (b1 & (1 << 2)) != 0,
            button_y: (b1 & (1 << 3)) != 0,

            button_left: (b1 & (1 << 4)) != 0,
            button_right: (b1 & (1 << 5)) != 0,
            button_down: (b1 & (1 << 6)) != 0,
            button_up: (b1 & (1 << 7)) != 0,

            button_start: (b2 & (1 << 0)) != 0,
            button_z: (b2 & (1 << 1)) != 0,
            button_r: (b2 & (1 << 2)) != 0,
            button_l: (b2 & (1 << 3)) != 0,

            stick_x: payload[1 + (9 * chan) + 3],
            stick_y: payload[1 + (9 * chan) + 4],
            substick_x: payload[1 + (9 * chan) + 5],
            substick_y: payload[1 + (9 * chan) + 6],
            trigger_left: payload[1 + (9 * chan) + 7],
            trigger_right: payload[1 + (9 * chan) + 8],
        }
    }
}

impl Default for Input {
    fn default() -> Input {
        Input {
            button_a: false,
            button_b: false,
            button_x: false,
            button_y: false,

            button_left: false,
            button_right: false,
            button_down: false,
            button_up: false,

            button_start: false,
            button_z: false,
            button_r: false,
            button_l: false,

            stick_x: MAIN_STICK_CENTER_X,
            stick_y: MAIN_STICK_CENTER_Y,
            substick_x: C_STICK_CENTER_X,
            substick_y: C_STICK_CENTER_Y,
            trigger_left: 0,
            trigger_right: 0,
        }
    }
}
