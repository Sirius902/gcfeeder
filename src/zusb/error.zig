const c = @import("c.zig");

pub const Error = error{
    Io,
    InvalidParam,
    Access,
    NoDevice,
    NotFound,
    Busy,
    Timeout,
    Overflow,
    Pipe,
    Interrupted,
    OutOfMemory,
    NotSupported,
    BadDescriptor,
    Other,
};

pub fn errorFromLibusb(err: c_int) Error {
    return switch (err) {
        c.LIBUSB_ERROR_IO => Error.Io,
        c.LIBUSB_ERROR_INVALID_PARAM => Error.InvalidParam,
        c.LIBUSB_ERROR_ACCESS => Error.Access,
        c.LIBUSB_ERROR_NO_DEVICE => Error.NoDevice,
        c.LIBUSB_ERROR_NOT_FOUND => Error.NotFound,
        c.LIBUSB_ERROR_BUSY => Error.Busy,
        c.LIBUSB_ERROR_TIMEOUT => Error.Timeout,
        c.LIBUSB_ERROR_OVERFLOW => Error.Overflow,
        c.LIBUSB_ERROR_PIPE => Error.Pipe,
        c.LIBUSB_ERROR_INTERRUPTED => Error.Interrupted,
        c.LIBUSB_ERROR_NO_MEM => Error.OutOfMemory,
        c.LIBUSB_ERROR_NOT_SUPPORTED => Error.NotSupported,
        c.LIBUSB_ERROR_OTHER => Error.Other,
        else => Error.Other,
    };
}

pub fn failable(err: c_int) Error!void {
    if (err != 0) {
        return errorFromLibusb(err);
    }
}
