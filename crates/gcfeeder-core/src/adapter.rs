use gcinput::{Input, Rumble, Stick};
use nusb::transfer::{ControlOut, ControlType, Recipient, RequestBuffer};
use tracing::{debug, trace};

const VID: u16 = 0x057E;
const PID: u16 = 0x0337;

const INPUT_PAYLOAD_LEN: usize = 37;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no device")]
    NoDevice,
    #[error("the device was disconnected")]
    Disconnected,
    #[error("device is missing endpoints")]
    MissingEndpoints,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("nusb transfer error: {0}")]
    NusbTransfer(#[from] nusb::transfer::TransferError),
}

struct Endpoints {
    pub in_: u8,
    pub out: u8,
}

pub struct Adapter {
    interface: nusb::Interface,
    endpoints: Endpoints,
}

impl Adapter {
    // TODO(Sirius902) Replace with `open_device`?
    pub async fn open() -> Result<Self> {
        trace!("Opening adapter...");

        // FUTURE(Sirius902) Use `watch_devices`.
        let mut device: Option<nusb::Device> = None;
        for device_info in nusb::list_devices()? {
            if Self::is_candidate(&device_info) {
                debug!("Adapter candidate found");

                // FUTURE(Sirius902) Don't fail the function if the adapter fails to open, try
                // others.
                device = Some(device_info.open()?);
                break;
            }
        }

        let device = device.ok_or(Error::NoDevice)?;
        let interface = device.detach_and_claim_interface(0)?;

        let endpoints = Self::find_endpoints(&interface)?;

        // From Dolphin:
        // This call makes Nyko-brand (and perhaps other) adapters work.
        // However it returns LIBUSB_ERROR_PIPE with Mayflash adapters.
        interface
            .control_out(ControlOut {
                control_type: ControlType::Class,
                recipient: Recipient::Interface,
                request: 11,
                value: 0x0001,
                index: 0x0,
                data: &[],
            })
            .await
            .into_result()?;

        // Initialize writing controller rumble.
        interface
            .interrupt_out(endpoints.out, vec![0x13])
            .await
            .into_result()?;

        let adapter = Self {
            interface,
            endpoints,
        };

        // Reset rumble just in case the adapter was rumbling previously.
        adapter.reset_rumble().await?;

        debug!("Connected to adapter");

        Ok(adapter)
    }

    pub async fn try_open(device_info: &nusb::DeviceInfo) -> Result<Self> {
        if !Self::is_candidate(device_info) {
            // TODO(Sirius902) A different error probably makes more sense here.
            return Err(Error::NoDevice);
        }

        trace!("Attempting to open adapter...");

        let device = device_info.open()?;

        let interface = device.detach_and_claim_interface(0)?;
        let endpoints = Self::find_endpoints(&interface)?;

        // From Dolphin:
        // This call makes Nyko-brand (and perhaps other) adapters work.
        // However it returns LIBUSB_ERROR_PIPE with Mayflash adapters.
        interface
            .control_out(ControlOut {
                control_type: ControlType::Class,
                recipient: Recipient::Interface,
                request: 11,
                value: 0x0001,
                index: 0x0,
                data: &[],
            })
            .await
            .into_result()?;

        // Initialize writing controller rumble.
        interface
            .interrupt_out(endpoints.out, vec![0x13])
            .await
            .into_result()?;

        let adapter = Self {
            interface,
            endpoints,
        };

        // Reset rumble just in case the adapter was rumbling previously.
        adapter.reset_rumble().await?;

        debug!("Connected to adapter");

        Ok(adapter)
    }

    pub async fn read_inputs(&self) -> Result<[Option<Input>; Port::COUNT]> {
        let payload = self
            .interface
            .interrupt_in(self.endpoints.in_, RequestBuffer::new(INPUT_PAYLOAD_LEN))
            .await
            .into_result()
            .map_err(|err| match err {
                nusb::transfer::TransferError::Disconnected => Error::Disconnected,
                _ => err.into(),
            })?;

        Ok(inputs_from_payload(&payload))
    }

    pub async fn write_rumble(&self, states: [Rumble; Port::COUNT]) -> Result<()> {
        self.interface
            .interrupt_out(
                self.endpoints.out,
                vec![
                    0x11,
                    states[0].into(),
                    states[1].into(),
                    states[2].into(),
                    states[3].into(),
                ],
            )
            .await
            .into_result()
            .map_err(|err| match err {
                nusb::transfer::TransferError::Disconnected => Error::Disconnected,
                _ => err.into(),
            })?;

        Ok(())
    }

    pub async fn reset_rumble(&self) -> Result<()> {
        debug!("Resetting rumble");
        self.write_rumble([Rumble::Off; Port::COUNT]).await
    }

    #[must_use]
    pub fn is_candidate(device_info: &nusb::DeviceInfo) -> bool {
        device_info.vendor_id() == VID && device_info.product_id() == PID
    }

    fn find_endpoints(interface: &nusb::Interface) -> Result<Endpoints> {
        let mut in_: Option<u8> = None;
        let mut out: Option<u8> = None;

        for descriptor in interface.descriptors() {
            for endpoint in descriptor.endpoints() {
                match endpoint.direction() {
                    nusb::transfer::Direction::In => in_ = Some(endpoint.address()),
                    nusb::transfer::Direction::Out => out = Some(endpoint.address()),
                }
            }
        }

        Ok(Endpoints {
            in_: in_.ok_or(Error::MissingEndpoints)?,
            out: out.ok_or(Error::MissingEndpoints)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Port {
    One,
    Two,
    Three,
    Four,
}

impl Port {
    pub const COUNT: usize = Self::all().len();

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
            Self::Three => 2,
            Self::Four => 3,
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::One, Self::Two, Self::Three, Self::Four]
    }
}

impl From<Port> for usize {
    fn from(port: Port) -> Self {
        port.index()
    }
}

impl TryFrom<usize> for Port {
    type Error = FromPortError;

    fn try_from(src: usize) -> std::result::Result<Self, Self::Error> {
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

fn inputs_from_payload(payload: &[u8]) -> [Option<Input>; Port::COUNT] {
    assert!(payload.len() == INPUT_PAYLOAD_LEN);

    let mut inputs = [None; Port::COUNT];

    for port in Port::all() {
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
