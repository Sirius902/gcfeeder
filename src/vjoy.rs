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
    pub b_device: BYTE, // Index of device. 1-based.
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

#[link(name = "vJoyInterface")]
extern "C" {
    pub fn vJoyEnabled() -> BOOL;
    pub fn DriverMatch(DllVer: *mut WORD, DrvVer: *mut WORD) -> BOOL;
    pub fn GetVJDStatus(rID: UINT) -> VjdStat;
    pub fn AcquireVJD(rID: UINT) -> BOOL;
    pub fn RelinquishVJD(rID: UINT) -> BOOL;
    pub fn UpdateVJD(rID: UINT, pData: *const JoystickPosition) -> BOOL;
    pub fn GetVJDButtonNumber(rID: UINT) -> i32;

    pub fn ResetVJD(rID: UINT) -> BOOL;
}
