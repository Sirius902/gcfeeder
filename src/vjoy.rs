use winapi::shared::minwindef::*;
use winapi::um::winnt::*;

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
}

impl JoystickPosition {
    pub fn zeroed() -> JoystickPosition {
        unsafe { std::mem::zeroed() }
    }
}

mod ffi {
    use super::*;
    use libc::c_int;

    #[allow(dead_code)]
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
    }
}

pub type DeviceId = u32;

#[derive(Debug)]
pub enum Error {
    Driver,
    Acquire(DeviceId),
    Device { id: DeviceId, status: VjdStat },
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
        unsafe {
            if !driver_enabled() {
                return Err(Error::Driver);
            }

            let status = ffi::GetVJDStatus(id);

            match status {
                VjdStat::Own | VjdStat::Free => {
                    if ffi::AcquireVJD(id) == FALSE {
                        return Err(Error::Acquire(id));
                    }
                }
                _ => return Err(Error::Device { id, status }),
            }
        }

        Ok(Device { id })
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
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if let VjdStat::Own = ffi::GetVJDStatus(self.id) {
                let _ = ffi::RelinquishVJD(self.id);
            }
        }
    }
}

pub fn driver_enabled() -> bool {
    unsafe { ffi::vJoyEnabled() != FALSE }
}
