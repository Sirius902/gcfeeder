const c = @import("c.zig");
const std = @import("std");
const Context = @import("context.zig").Context;
const Device = @import("device.zig").Device;
const fromLibusb = @import("constructor.zig").fromLibusb;

usingnamespace @import("error.zig");

pub const DeviceHandle = struct {
    ctx: *Context,
    raw: *c.libusb_device_handle,
    interfaces: u256,

    pub fn deinit(self: DeviceHandle) void {
        var iface: u9 = 0;
        while (iface < 256) : (iface += 1) {
            if ((self.interfaces >> @truncate(u8, iface)) & 1 == 1) {
                _ = c.libusb_release_interface(self.raw, @as(c_int, iface));
            }
        }

        c.libusb_close(self.raw);
    }

    pub fn claimInterface(self: *DeviceHandle, iface: u8) Error!void {
        try failable(c.libusb_claim_interface(self.raw, @as(c_int, iface)));
        self.interfaces |= @as(u256, 1) << iface;
    }

    pub fn releaseInterface(self: *DeviceHandle, iface: u8) Error!void {
        try failable(c.libusb_release_interface(self.raw, @as(c_int, iface)));
        self.interfaces &= ~(@as(u256, 1) << iface);
    }

    pub fn device(self: DeviceHandle) Device {
        return fromLibusb(Device, .{ self.ctx, c.libusb_get_device(self.raw).? });
    }

    pub fn writeControl(
        self: DeviceHandle,
        requestType: u8,
        request: u8,
        value: u16,
        index: u16,
        buf: []const u8,
        timeout_ms: u64,
    ) (error{Overflow} || Error)!usize {
        if (requestType & c.LIBUSB_ENDPOINT_DIR_MASK != c.LIBUSB_ENDPOINT_OUT) {
            return Error.InvalidParam;
        }

        const res = c.libusb_control_transfer(
            self.raw,
            requestType,
            request,
            value,
            index,
            @intToPtr([*c]u8, @ptrToInt(buf.ptr)),
            try std.math.cast(u16, buf.len),
            try std.math.cast(c_uint, timeout_ms),
        );

        if (res < 0) {
            return errorFromLibusb(res);
        } else {
            return std.math.cast(usize, res) catch unreachable;
        }
    }

    pub fn readInterrupt(
        self: DeviceHandle,
        endpoint: u8,
        buf: []u8,
        timeout_ms: u64,
    ) (error{Overflow} || Error)!usize {
        if (endpoint & c.LIBUSB_ENDPOINT_DIR_MASK != c.LIBUSB_ENDPOINT_IN) {
            return Error.InvalidParam;
        }

        var transferred: c_int = undefined;

        const ret = c.libusb_interrupt_transfer(
            self.raw,
            endpoint,
            buf.ptr,
            try std.math.cast(c_int, buf.len),
            &transferred,
            try std.math.cast(c_uint, timeout_ms),
        );

        if (ret == 0 or ret == c.LIBUSB_ERROR_INTERRUPTED and transferred > 0) {
            return std.math.cast(usize, transferred) catch unreachable;
        } else {
            return errorFromLibusb(ret);
        }
    }

    pub fn writeInterrupt(
        self: DeviceHandle,
        endpoint: u8,
        buf: []const u8,
        timeout_ms: u64,
    ) (error{Overflow} || Error)!usize {
        if (endpoint & c.LIBUSB_ENDPOINT_DIR_MASK != c.LIBUSB_ENDPOINT_OUT) {
            return Error.InvalidParam;
        }

        var transferred: c_int = undefined;

        const ret = c.libusb_interrupt_transfer(
            self.raw,
            endpoint,
            @intToPtr([*c]u8, @ptrToInt(buf.ptr)),
            try std.math.cast(c_int, buf.len),
            &transferred,
            try std.math.cast(c_uint, timeout_ms),
        );

        if (ret == 0 or ret == c.LIBUSB_ERROR_INTERRUPTED and transferred > 0) {
            return std.math.cast(usize, transferred) catch unreachable;
        } else {
            return errorFromLibusb(ret);
        }
    }
};
