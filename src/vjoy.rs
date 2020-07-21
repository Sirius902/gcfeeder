use libc::*;

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
    pub b_device: c_uchar, // Index of device. 1-based.
    pub w_throttle: c_long,
    pub w_rudder: c_long,
    pub w_aileron: c_long,
    pub w_axis_x: c_long,
    pub w_axis_y: c_long,
    pub w_axis_z: c_long,
    pub w_axis_x_rot: c_long,
    pub w_axis_y_rot: c_long,
    pub w_axis_z_rot: c_long,
    pub w_slider: c_long,
    pub w_dial: c_long,
    pub w_wheel: c_long,
    pub w_axis_vx: c_long,
    pub w_axis_vy: c_long,
    pub w_axis_vz: c_long,
    pub w_axis_vbrx: c_long,
    pub w_axis_vbry: c_long,
    pub w_axis_vbrz: c_long,
    pub l_buttons: c_long, // 32 buttons: 0x00000001 means button1 is pressed, 0x80000000 -> button32 is pressed
    pub b_hats: c_ulong,   // Lower 4 bits: HAT switch or 16-bit of continuous HAT switch
    pub b_hats_ex1: c_ulong, // 16-bit of continuous HAT switch
    pub b_hats_ex2: c_ulong, // 16-bit of continuous HAT switch
    pub b_hats_ex3: c_ulong, // 16-bit of continuous HAT switch
}

impl JoystickPosition {
    pub fn zeroed() -> JoystickPosition {
        unsafe { std::mem::zeroed() }
    }
}

#[link(name = "vJoyInterface")]
extern "C" {
    pub fn vJoyEnabled() -> bool;
    pub fn DriverMatch(DllVer: *mut c_ushort, DrvVer: *mut c_ushort) -> bool;
    pub fn GetVJDStatus(rID: c_uint) -> VjdStat;
    pub fn AcquireVJD(rID: c_uint) -> bool;
    pub fn RelinquishVJD(rID: c_uint) -> bool;
    pub fn UpdateVJD(rID: c_uint, pData: *const JoystickPosition) -> bool;
    pub fn GetVJDButtonNumber(rID: c_uint) -> i32;

    pub fn ResetVJD(rID: c_uint) -> bool;
}
