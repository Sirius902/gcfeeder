const std = @import("std");
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

pub const Context = struct {
    ctx: *c.libusb_context,

    pub fn init() Error!Context {
        var ctx_opt: ?*c.libusb_context = null;
        try failable(c.libusb_init(&ctx_opt));

        return Context{ .ctx = ctx_opt.? };
    }

    pub fn deinit(self: Context) void {
        _ = c.libusb_exit(self.ctx);
    }

    pub fn openDeviceWithVidPid(
        self: Context,
        vendor_id: u16,
        product_id: u16,
    ) Error!?DeviceHandle {
        if (c.libusb_open_device_with_vid_pid(self.ctx, vendor_id, product_id)) |handle| {
            return DeviceHandle{
                .ctx = self.ctx,
                .handle = handle,
                .interfaces = 0,
            };
        } else {
            return null;
        }
    }
};

pub const Direction = enum {
    In,
    Out,
};

pub const EndpointDescriptor = struct {
    descriptor: *const c.libusb_endpoint_descriptor,

    pub fn direction(self: EndpointDescriptor) Direction {
        return switch (self.descriptor.*.bEndpointAddress & c.LIBUSB_ENDPOINT_DIR_MASK) {
            c.LIBUSB_ENDPOINT_OUT => Direction.Out,
            c.LIBUSB_ENDPOINT_IN => Direction.In,
            else => Direction.In,
        };
    }

    pub fn number(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress & 0x07;
    }

    pub fn address(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress;
    }
};

pub const EndpointDescriptors = struct {
    iter: []const c.libusb_endpoint_descriptor,
    index: usize,

    pub fn next(self: *EndpointDescriptors) ?EndpointDescriptor {
        if (self.index < self.iter.len) {
            defer self.index += 1;
            return EndpointDescriptor{ .descriptor = &self.iter[self.index] };
        } else {
            return null;
        }
    }
};

pub const InterfaceDescriptor = struct {
    descriptor: *const c.libusb_interface_descriptor,

    pub fn endpointDescriptors(self: InterfaceDescriptor) EndpointDescriptors {
        return EndpointDescriptors{
            .iter = self.descriptor.*.endpoint[0..self.descriptor.*.bNumEndpoints],
            .index = 0,
        };
    }
};

pub const InterfaceDescriptors = struct {
    iter: []const c.libusb_interface_descriptor,
    index: usize,

    pub fn next(self: *InterfaceDescriptors) ?InterfaceDescriptor {
        if (self.index < self.iter.len) {
            defer self.index += 1;
            return InterfaceDescriptor{ .descriptor = &self.iter[self.index] };
        } else {
            return null;
        }
    }
};

pub const Interface = struct {
    iter: []const c.libusb_interface_descriptor,

    pub fn number(self: Interface) u8 {
        return self.iter[0].bInterfaceNumber;
    }

    pub fn descriptors(self: Interface) InterfaceDescriptors {
        return InterfaceDescriptors{
            .iter = self.iter,
            .index = 0,
        };
    }
};

pub const Interfaces = struct {
    interfaces: []const c.libusb_interface,
    index: usize,

    pub fn next(self: *Interfaces) ?Interface {
        if (self.index < self.interfaces.len) {
            defer self.index += 1;

            const len = std.math.cast(
                usize,
                self.interfaces[self.index].num_altsetting,
            ) catch unreachable;

            return Interface{
                .iter = self.interfaces[self.index].altsetting[0..len],
            };
        } else {
            return null;
        }
    }
};

pub const ConfigDescriptor = struct {
    descriptor: *c.libusb_config_descriptor,

    pub fn deinit(self: ConfigDescriptor) void {
        _ = c.libusb_free_config_descriptor(self.descriptor);
    }

    pub fn interfaces(self: ConfigDescriptor) Interfaces {
        return Interfaces{
            .interfaces = self.descriptor.*.interface[0..self.descriptor.*.bNumInterfaces],
            .index = 0,
        };
    }
};

pub const Device = struct {
    ctx: *c.libusb_context,
    device: *c.libusb_device,

    pub fn deinit(self: Device) void {
        _ = c.libusb_unref_device(self.device);
    }

    pub fn configDescriptor(self: Device, config_index: u8) Error!ConfigDescriptor {
        var descriptor_opt: ?*c.libusb_config_descriptor = null;

        try failable(c.libusb_get_config_descriptor(
            self.device,
            config_index,
            &descriptor_opt,
        ));

        if (descriptor_opt) |descriptor| {
            return ConfigDescriptor{ .descriptor = descriptor };
        } else {
            unreachable;
        }
    }

    fn fromLibusb(ctx: *c.libusb_context, device: *c.libusb_device) Device {
        _ = c.libusb_ref_device(device);
        return Device{ .ctx = ctx, .device = device };
    }
};

pub const DeviceHandle = struct {
    ctx: *c.libusb_context,
    handle: *c.libusb_device_handle,
    interfaces: u256,

    pub fn deinit(self: DeviceHandle) void {
        var iface: u9 = 0;
        while (iface < 256) : (iface += 1) {
            if ((self.interfaces >> @truncate(u8, iface)) & 1 == 1) {
                _ = c.libusb_release_interface(self.handle, @as(c_int, iface));
            }
        }

        c.libusb_close(self.handle);
    }

    pub fn claimInterface(self: *DeviceHandle, iface: u8) Error!void {
        try failable(c.libusb_claim_interface(self.handle, @as(c_int, iface)));
        self.interfaces |= @as(u256, 1) << iface;
    }

    pub fn releaseInterface(self: *DeviceHandle, iface: u8) Error!void {
        try failable(c.libusb_release_interface(self.handle, @as(c_int, iface)));
        self.interfaces &= ~(@as(u256, 1) << iface);
    }

    pub fn device(self: DeviceHandle) Device {
        if (c.libusb_get_device(self.handle)) |d| {
            return Device.fromLibusb(self.ctx, d);
        } else {
            unreachable;
        }
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
            self.handle,
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
            self.handle,
            endpoint,
            buf.ptr,
            try std.math.cast(c_int, buf.len),
            &transferred,
            try std.math.cast(c_uint, timeout_ms),
        );

        const utrans = std.math.cast(usize, transferred) catch unreachable;

        return switch (ret) {
            0 => utrans,
            c.LIBUSB_ERROR_INTERRUPTED => if (transferred > 0) utrans else errorFromLibusb(ret),
            else => errorFromLibusb(ret),
        };
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
            self.handle,
            endpoint,
            @intToPtr([*c]u8, @ptrToInt(buf.ptr)),
            try std.math.cast(c_int, buf.len),
            &transferred,
            try std.math.cast(c_uint, timeout_ms),
        );

        const utrans = std.math.cast(usize, transferred) catch unreachable;

        return switch (ret) {
            0 => utrans,
            c.LIBUSB_ERROR_INTERRUPTED => if (transferred > 0) utrans else errorFromLibusb(ret),
            else => errorFromLibusb(ret),
        };
    }
};

fn errorFromLibusb(err: c_int) Error {
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

fn failable(err: c_int) Error!void {
    if (err != 0) {
        return errorFromLibusb(err);
    }
}
