use std::{ops::ControlFlow, result, time::Duration};

use enum_iterator::{all, cardinality, Sequence};
use gcinput::{Input, Rumble, Stick};
use log::info;
use rusb::{DeviceHandle, UsbContext};
use thiserror::Error;

pub mod poller;

const VID: u16 = 0x057E;
const PID: u16 = 0x0337;

const PAYLOAD_LEN: usize = 37;

const ALLOWED_TIMEOUT: Duration = Duration::from_millis(16);

#[derive(Error, Debug)]
pub enum Error {
    #[error("usb error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("no device")]
    NoDevice,
    #[error("invalid payload")]
    InvalidPayload,
}

pub type Result<T> = result::Result<T, Error>;

struct Endpoints {
    pub in_: u8,
    pub out: u8,
}

pub struct Adapter<T: UsbContext> {
    handle: rusb::DeviceHandle<T>,
    endpoints: Endpoints,
}

impl<T: UsbContext> Adapter<T> {
    pub fn open(context: &T) -> Result<Self> {
        let mut handle = Self::find_and_open_device(context)?;

        match handle.kernel_driver_active(0) {
            Ok(b) => {
                if b {
                    handle.detach_kernel_driver(0)?;
                }
                Ok(())
            }
            Err(rusb::Error::NotSupported) => Ok(()),
            Err(e) => Err(e),
        }?;

        handle.claim_interface(0)?;

        let endpoints = Self::find_endpoints(&handle)?;

        // From Dolphin:
        // This call makes Nyko-brand (and perhaps other) adapters work.
        // However it returns LIBUSB_ERROR_PIPE with Mayflash adapters.
        let _ = handle.write_control(0x21, 11, 0x0001, 0, &[], Duration::from_secs(1))?;

        // Not sure what this does but Dolphin does it
        let _ = handle.write_interrupt(endpoints.out, &[0x13], ALLOWED_TIMEOUT)?;

        info!("Connected to adapter");

        Ok(Self { handle, endpoints })
    }

    pub fn read_inputs(&self) -> Result<[Option<Input>; Port::COUNT]> {
        let mut payload = [0_u8; PAYLOAD_LEN];
        let bytes_read =
            self.handle
                .read_interrupt(self.endpoints.in_, &mut payload, ALLOWED_TIMEOUT)?;

        if bytes_read == PAYLOAD_LEN && payload[0] == rusb::constants::LIBUSB_DT_HID {
            Ok(inputs_from_payload(&payload))
        } else {
            Err(Error::InvalidPayload)
        }
    }

    pub fn write_rumble(&self, states: [Rumble; Port::COUNT]) -> Result<()> {
        let payload = [
            0x11,
            states[0].into(),
            states[1].into(),
            states[2].into(),
            states[3].into(),
        ];

        _ = self
            .handle
            .write_interrupt(self.endpoints.out, &payload, ALLOWED_TIMEOUT)?;

        Ok(())
    }

    pub fn reset_rumble(&self) -> Result<()> {
        self.write_rumble([Rumble::Off; Port::COUNT])
    }

    fn find_and_open_device(context: &T) -> Result<DeviceHandle<T>> {
        let handle = context
            .devices()?
            .iter()
            .filter(|device| {
                device
                    .device_descriptor()
                    .map(|descriptor| {
                        descriptor.vendor_id() == VID && descriptor.product_id() == PID
                    })
                    .unwrap_or(false)
            })
            .try_fold(Err(Error::NoDevice), |acc, device| {
                if acc.is_err() {
                    ControlFlow::Continue(device.open().map_err(Error::Usb))
                } else {
                    ControlFlow::Break(acc)
                }
            });

        match handle {
            ControlFlow::Continue(c) => c,
            ControlFlow::Break(b) => b,
        }
    }

    /// Returns (`in_endpoint`, `out_endpoint`) if found, and if not, an error.
    fn find_endpoints(handle: &rusb::DeviceHandle<T>) -> rusb::Result<Endpoints> {
        let device = handle.device();
        let config = device.config_descriptor(0)?;

        let mut in_ = 0_u8;
        let mut out = 0_u8;

        for iface in config.interfaces() {
            for descriptor in iface.descriptors() {
                for endpoint in descriptor.endpoint_descriptors() {
                    match endpoint.direction() {
                        rusb::Direction::In => {
                            in_ = endpoint.address();
                        }
                        rusb::Direction::Out => {
                            out = endpoint.address();
                        }
                    }
                }
            }
        }

        Ok(Endpoints { in_, out })
    }
}

impl<T: UsbContext> Drop for Adapter<T> {
    fn drop(&mut self) {
        let _ = self.reset_rumble();
        info!("Disconnected from adapter");
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Sequence)]
pub enum Port {
    One,
    Two,
    Three,
    Four,
}

impl Port {
    pub const COUNT: usize = cardinality::<Self>();

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
            Self::Three => 2,
            Self::Four => 3,
        }
    }
}

impl From<Port> for usize {
    fn from(port: Port) -> Self {
        port.index()
    }
}

impl TryFrom<usize> for Port {
    type Error = FromPortError;

    fn try_from(src: usize) -> result::Result<Self, Self::Error> {
        match src {
            0 => Ok(Self::One),
            1 => Ok(Self::Two),
            2 => Ok(Self::Three),
            3 => Ok(Self::Four),
            _ => Err(FromPortError::OutOfRange),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FromPortError {
    #[error("source out of range")]
    OutOfRange,
}

fn inputs_from_payload(payload: &[u8; PAYLOAD_LEN]) -> [Option<Input>; Port::COUNT] {
    let mut inputs = [None; Port::COUNT];

    for port in all::<Port>() {
        let index = port.index();
        // type is 0 if no controller is plugged, 1 if wired, and 2 if wireless
        let controller_type = payload[1 + (9 * index)] >> 4;
        let connected = controller_type != 0;

        if connected {
            let b1 = payload[1 + (9 * index) + 1];
            let b2 = payload[1 + (9 * index) + 2];

            inputs[index] = Some(Input {
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

                main_stick: Stick::new(payload[1 + (9 * index) + 3], payload[1 + (9 * index) + 4]),
                c_stick: Stick::new(payload[1 + (9 * index) + 5], payload[1 + (9 * index) + 6]),
                left_trigger: payload[1 + (9 * index) + 7],
                right_trigger: payload[1 + (9 * index) + 8],
            });
        }
    }

    inputs
}
