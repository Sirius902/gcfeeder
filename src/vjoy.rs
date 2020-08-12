use channel::Sender;
use crossbeam::channel;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use winapi::shared::minwindef::*;
use winapi::um::winnt::*;

static FFB_SENDER: Lazy<Mutex<Option<Sender<FFBPacket>>>> = Lazy::new(|| Mutex::new(None));

pub type FFBPacket = (DeviceId, FFBOp);

#[repr(C)]
#[derive(Debug)]
pub enum VjdStat {
    Own,  // The vJoy Device is owned by this application.
    Free, // The vJoy Device is NOT owned by any application (including this one).
    Busy, // The vJoy Device is owned by another application. It cannot be acquired by this application.
    Miss, // The vJoy Device is missing. It either does not exist or the driver is down.
    Unknown,
}

#[repr(C)]
pub struct JoystickPosition {
    /// Set by the `Device` before updating position.
    pub(self) b_device: BYTE, // Index of device. 1-based.
    pub w_throttle: LONG,
    pub w_rudder: LONG,
    pub w_aileron: LONG,
    pub w_axis_x: LONG,
    pub w_axis_y: LONG,
    pub w_axis_z: LONG,
    pub w_axis_x_rot: LONG,
    pub w_axis_y_rot: LONG,
    pub w_axis_z_rot: LONG,
    pub w_slider: LONG,
    pub w_dial: LONG,
    pub w_wheel: LONG,
    pub w_axis_vx: LONG,
    pub w_axis_vy: LONG,
    pub w_axis_vz: LONG,
    pub w_axis_vbrx: LONG,
    pub w_axis_vbry: LONG,
    pub w_axis_vbrz: LONG,
    pub l_buttons: LONG, // 32 buttons: 0x00000001 means button1 is pressed, 0x80000000 -> button32 is pressed
    pub b_hats: DWORD,   // Lower 4 bits: HAT switch or 16-bit of continuous HAT switch
    pub b_hats_ex1: DWORD, // 16-bit of continuous HAT switch
    pub b_hats_ex2: DWORD, // 16-bit of continuous HAT switch
    pub b_hats_ex3: DWORD, // 16-bit of continuous HAT switch

    // JOYSTICK_POSITION_V2 Extension
    pub l_buttons_ex1: LONG, // Buttons 33-64
    pub l_buttons_ex2: LONG, // Buttons 65-96
    pub l_buttons_ex3: LONG, // Buttons 97-128
}

impl JoystickPosition {
    pub fn zeroed() -> JoystickPosition {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FFBOp {
    Start = 1,
    Solo = 2,
    Stop = 3,
}

#[repr(C)]
#[derive(Debug)]
pub struct FFBEffOp {
    pub effect_block_index: BYTE,
    pub effect_op: FFBOp,
    pub loop_count: BYTE,
}

#[repr(C)]
#[derive(Debug)]
pub enum FFBPType {
    // Write
    EffectReport = 0x01,          // Usage Set Effect Report
    EnvelopeReport = 0x02,        // Usage Set Envelope Report
    ConditionReport = 0x03,       // Usage Set Condition Report
    PeriodicReport = 0x04,        // Usage Set Periodic Report
    ConstantReport = 0x05,        // Usage Set Constant Force Report
    RampReport = 0x06,            // Usage Set Ramp Force Report
    CustomReport = 0x07,          // Usage Custom Force Data Report
    DownloadSample = 0x08,        // Usage Download Force Sample
    EffectOperationReport = 0x09, // Usage Effect Operation Report
    BlockFreeReport = 0x0A,       // Usage PID Block Free Report
    ControlReport = 0x0B,         // Usage PID Device Control
    GainReport = 0x0C,            // Usage Device Gain Report
    SetCustomReport = 0x0D,       // Usage Set Custom Force Report

    // Feature
    NewEffectReport = 0x01 + 0x10, // Usage Create New Effect Report
    BlockLoadReport = 0x02 + 0x10, // Usage Block Load Report
    PoolReport = 0x03 + 0x10,      // Usage PID Pool Report
}

#[allow(dead_code)]
mod ffi {
    use super::*;
    use libc::c_int;

    pub type FFBData = VOID;
    pub type FfbGenCB = extern "C" fn(ffb_data: *const FFBData, userdata: *mut VOID);

    #[link(name = "vJoyInterface")]
    extern "C" {
        pub fn vJoyEnabled() -> BOOL;
        pub fn DriverMatch(DllVer: *mut WORD, DrvVer: *mut WORD) -> BOOL;
        pub fn GetVJDStatus(rID: UINT) -> VjdStat;
        pub fn AcquireVJD(rID: UINT) -> BOOL;
        pub fn RelinquishVJD(rID: UINT) -> BOOL;
        pub fn UpdateVJD(rID: UINT, pData: *const JoystickPosition) -> BOOL;
        pub fn GetVJDButtonNumber(rID: UINT) -> c_int;

        pub fn ResetVJD(rID: UINT) -> BOOL;

        pub fn FfbRegisterGenCB(cb: FfbGenCB, data: *mut VOID);
        pub fn IsDeviceFfb(rID: UINT) -> BOOL;
        pub fn IsDeviceFfbEffect(rID: UINT, Effect: UINT) -> BOOL;

        pub fn Ffb_h_DeviceID(Packet: *const FFBData, DeviceId: *mut c_int) -> DWORD;
        pub fn Ffb_h_EffOp(Packet: *const FFBData, Operation: *mut FFBEffOp) -> DWORD;
        pub fn Ffb_h_Type(Packet: *const FFBData, Type: *mut FFBPType) -> DWORD;
    }
}

pub type DeviceId = u32;

#[derive(Debug)]
pub enum Error {
    Driver,
    Acquire { id: DeviceId, status: VjdStat },
    Relinquish { id: DeviceId, status: VjdStat },
    Update(DeviceId),
}

pub struct Device {
    id: DeviceId,
}

impl Device {
    /// Acquires the specified vJoy device.
    ///
    /// `id` is one indexed.
    pub fn acquire(id: DeviceId) -> Result<Device, Error> {
        assert!(1 <= id && id <= 15, "vjoy: device id out of range");
        unsafe {
            if !driver_enabled() {
                return Err(Error::Driver);
            }

            let status = ffi::GetVJDStatus(id);

            match status {
                VjdStat::Free if ffi::AcquireVJD(id) == TRUE => Ok(Device { id }),
                VjdStat::Own => Ok(Device { id }),
                _ => Err(Error::Acquire { id, status }),
            }
        }
    }

    pub fn relinquish(&mut self) -> Result<(), Error> {
        unsafe {
            let status = ffi::GetVJDStatus(self.id);

            if let VjdStat::Own = status {
                if ffi::RelinquishVJD(self.id) == TRUE {
                    return Ok(());
                }
            }

            Err(Error::Relinquish {
                id: self.id,
                status,
            })
        }
    }

    pub fn update(&self, mut position: JoystickPosition) -> Result<(), Error> {
        position.b_device = self.id as u8;

        if unsafe { ffi::UpdateVJD(self.id, &position) } == FALSE {
            Err(Error::Update(self.id))
        } else {
            Ok(())
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn status(id: DeviceId) -> VjdStat {
        unsafe { ffi::GetVJDStatus(id) }
    }

    pub fn button_count(id: DeviceId) -> i32 {
        unsafe { ffi::GetVJDButtonNumber(id) }
    }

    pub fn is_ffb(&self) -> bool {
        unsafe { ffi::IsDeviceFfb(self.id) == TRUE }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.relinquish();
    }
}

pub fn driver_enabled() -> bool {
    unsafe { ffi::vJoyEnabled() == TRUE }
}

pub fn start_ffb() -> channel::Receiver<FFBPacket> {
    let (ffb_sender, ffb_receiver) = channel::unbounded();
    *FFB_SENDER.lock().unwrap() = Some(ffb_sender);

    unsafe {
        ffi::FfbRegisterGenCB(update_ffb, 0 as *mut VOID);
    }

    ffb_receiver
}

#[no_mangle]
extern "C" fn update_ffb(ffb_data: *const ffi::FFBData, _: *mut VOID) {
    unsafe {
        let mut id = 0;
        let mut operation = std::mem::zeroed::<FFBEffOp>();

        if ffi::Ffb_h_DeviceID(ffb_data, &mut id) == ERROR_SEVERITY_SUCCESS
            && ffi::Ffb_h_EffOp(ffb_data, &mut operation) == ERROR_SEVERITY_SUCCESS
        {
            if let Some(ref ffb_sender) = *FFB_SENDER.lock().unwrap() {
                ffb_sender
                    .send((id as DeviceId, operation.effect_op))
                    .unwrap();
            }
        }
    }
}
