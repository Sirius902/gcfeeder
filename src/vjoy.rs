use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;
use winapi::shared::minwindef::*;
use winapi::um::winnt::*;

static FFB_STATUS: Lazy<RwLock<HashMap<DeviceId, FFBOp>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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
    effect_block_index: BYTE,
    effect_op: FFBOp,
    loop_count: BYTE,
}

#[allow(dead_code)]
mod ffi {
    use super::*;
    use libc::c_int;

    pub type FfbData = VOID;
    pub type FfbGenCB = extern "C" fn(ffb_data: *const FfbData, userdata: *mut VOID);

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

        pub fn Ffb_h_DeviceID(Packet: *const FfbData, DeviceId: *mut c_int) -> DWORD;
        pub fn Ffb_h_EffOp(Packet: *const FfbData, Operation: *mut FFBEffOp) -> DWORD;
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
        unsafe {
            if !driver_enabled() {
                return Err(Error::Driver);
            }

            let status = ffi::GetVJDStatus(id);

            match status {
                VjdStat::Free => {
                    if ffi::AcquireVJD(id) == TRUE {
                        return Ok(Device { id });
                    }
                }
                VjdStat::Own => {
                    return Ok(Device { id });
                }
                _ => {}
            }

            Err(Error::Acquire { id, status })
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

// TODO: I'm not sure if it's safe to call FfbRegisterGenCB more than once so maybe prevent that
pub fn start_ffb() {
    unsafe {
        ffi::FfbRegisterGenCB(update_ffb, 0 as *mut VOID);
    }
}

pub fn ffb_status(id: DeviceId) -> Option<FFBOp> {
    FFB_STATUS.read().unwrap().get(&id).cloned()
}

#[no_mangle]
extern "C" fn update_ffb(ffb_data: *const ffi::FfbData, _: *mut VOID) {
    unsafe {
        let mut id = 0;
        let mut operation = std::mem::zeroed::<FFBEffOp>();

        if ffi::Ffb_h_DeviceID(ffb_data, &mut id) == ERROR_SEVERITY_SUCCESS {
            if ffi::Ffb_h_EffOp(ffb_data, &mut operation) == ERROR_SEVERITY_SUCCESS {
                assert!(1 <= id && id <= 15, "ffb: device id out of range");
                let mut ffb_status = FFB_STATUS.write().unwrap();
                let _ = ffb_status.insert(id as DeviceId, operation.effect_op);
            }
        }
    }
}
