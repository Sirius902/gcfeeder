const std = @import("std");
const c = @import("c.zig");
const Allocator = std.mem.Allocator;

pub usingnamespace @import("error.zig");

pub const dt_hid = c.LIBUSB_DT_HID;

pub fn Transfer(comptime T: type) type {
    return struct {
        const Self = @This();

        const libusb_transfer = extern struct {
            dev_handle: *c.libusb_device_handle,
            flags: u8,
            endpoint: u8,
            @"type": u8,
            timeout: c_uint,
            status: c.libusb_transfer_status,
            length: c_int,
            actual_length: c_int,
            callback: fn (*libusb_transfer) callconv(.C) void,
            user_data: ?*c_void,
            buffer: [*c]u8,
            num_iso_packets: c_int,
            iso_packet_desc: [0]c.libusb_iso_packet_descriptor, // TODO: Variable Length Array
        };

        allocator: *Allocator,
        transfer: *libusb_transfer,
        user_data: T,
        callback: fn (*Self) void,

        pub fn deinit(self: *const Self) void {
            c.libusb_free_transfer(@ptrCast(*c.libusb_transfer, self.transfer));
            self.allocator.destroy(self);
        }

        pub fn submit(self: *Self) Error!void {
            self.transfer.user_data = self;
            try failable(c.libusb_submit_transfer(@ptrCast(*c.libusb_transfer, self.transfer)));
        }

        pub fn cancel(self: *Self) Error!void {
            try failable(c.libusb_cancel_transfer(@ptrCast(*c.libusb_transfer, self.transfer)));
        }

        pub fn buffer(self: Self) []u8 {
            const length = std.math.cast(usize, self.transfer.length) catch @panic("Buffer length too large");
            return self.transfer.buffer[0..length];
        }

        pub fn fillInterrupt(
            allocator: *Allocator,
            handle: *DeviceHandle,
            endpoint: u8,
            buf: []u8,
            callback: fn (*Self) void,
            user_data: T,
            timeout: u64,
        ) (Allocator.Error || Error)!*Self {
            const opt_transfer = @intToPtr(?*libusb_transfer, @ptrToInt(c.libusb_alloc_transfer(0)));

            if (opt_transfer) |transfer| {
                transfer.*.dev_handle = handle.handle;
                transfer.*.endpoint = endpoint;
                transfer.*.@"type" = c.LIBUSB_TRANSFER_TYPE_INTERRUPT;
                transfer.*.timeout = std.math.cast(c_uint, timeout) catch @panic("Timeout too large");
                transfer.*.buffer = buf.ptr;
                transfer.*.length = std.math.cast(c_int, buf.len) catch @panic("Length too large");
                transfer.*.callback = callbackRaw;

                var self = try allocator.create(Self);
                self.* = Self{
                    .allocator = allocator,
                    .transfer = transfer,
                    .user_data = user_data,
                    .callback = callback,
                };

                return self;
            } else {
                return Error.OutOfMemory;
            }
        }

        export fn callbackRaw(transfer: *libusb_transfer) void {
            const self = @intToPtr(*Self, @ptrToInt(transfer.user_data.?));
            self.callback(self);
        }
    };
}

pub const Context = struct {
    ctx: *c.libusb_context,

    pub fn init() Error!Context {
        var ctx: ?*c.libusb_context = null;
        try failable(c.libusb_init(&ctx));

        return Context{ .ctx = ctx.? };
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
            return DeviceHandle.fromLibusb(self.ctx, handle);
        } else {
            return null;
        }
    }
};

pub const Devices = struct {
    ctx: *Context,
    devices: []?*c.libusb_device,
    i: usize,

    pub fn next(self: *Devices) ?Device {
        if (self.i < self.devices.len) {
            defer self.i += 1;
            return Device.fromLibusb(self.ctx.ctx, self.devices[self.i].?);
        } else {
            return null;
        }
    }
};

pub const DeviceList = struct {
    ctx: *Context,
    list: [*c]?*c.libusb_device,
    len: usize,

    pub fn init(ctx: *Context) !DeviceList {
        var list: [*c]?*c.libusb_device = undefined;
        const n = c.libusb_get_device_list(ctx.ctx, &list);

        if (n < 0) {
            return errorFromLibusb(
                std.math.cast(c_int, n) catch unreachable,
            );
        } else {
            return DeviceList{
                .ctx = ctx,
                .list = list,
                .len = std.math.cast(usize, n) catch unreachable,
            };
        }
    }

    pub fn deinit(self: DeviceList) void {
        c.libusb_free_device_list(self.list, 1);
    }

    pub fn devices(self: DeviceList) Devices {
        return Devices{
            .ctx = self.ctx,
            .devices = self.list[0..self.len],
            .i = 0,
        };
    }
};

pub const Direction = enum {
    In,
    Out,
};

pub const TransferType = enum {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
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

    pub fn transferType(self: EndpointDescriptor) TransferType {
        return switch (self.descriptor.*.bmAttributes & c.LIBUSB_TRANSFER_TYPE_MASK) {
            c.LIBUSB_TRANSFER_TYPE_CONTROL => TransferType.Control,
            c.LIBUSB_TRANSFER_TYPE_ISOCHRONOUS => TransferType.Isochronous,
            c.LIBUSB_TRANSFER_TYPE_BULK => TransferType.Bulk,
            c.LIBUSB_TRANSFER_TYPE_INTERRUPT => TransferType.Interrupt,
            else => TransferType.Interrupt,
        };
    }

    pub fn number(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress & 0x07;
    }

    pub fn address(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bEndpointAddress;
    }

    pub fn interval(self: EndpointDescriptor) u8 {
        return self.descriptor.*.bInterval;
    }
};

pub const EndpointDescriptors = struct {
    iter: []const c.libusb_endpoint_descriptor,
    i: usize,

    pub fn next(self: *EndpointDescriptors) ?EndpointDescriptor {
        if (self.i < self.iter.len) {
            defer self.i += 1;
            return EndpointDescriptor{ .descriptor = &self.iter[self.i] };
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
            .i = 0,
        };
    }
};

pub const InterfaceDescriptors = struct {
    iter: []const c.libusb_interface_descriptor,
    i: usize,

    pub fn next(self: *InterfaceDescriptors) ?InterfaceDescriptor {
        if (self.i < self.iter.len) {
            defer self.i += 1;
            return InterfaceDescriptor{ .descriptor = &self.iter[self.i] };
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
            .i = 0,
        };
    }
};

pub const Interfaces = struct {
    interfaces: []const c.libusb_interface,
    i: usize,

    pub fn next(self: *Interfaces) ?Interface {
        if (self.i < self.interfaces.len) {
            defer self.i += 1;

            const len = std.math.cast(
                usize,
                self.interfaces[self.i].num_altsetting,
            ) catch unreachable;

            return Interface{
                .iter = self.interfaces[self.i].altsetting[0..len],
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
            .i = 0,
        };
    }
};

pub const DeviceDescriptor = struct {
    descriptor: c.libusb_device_descriptor,

    pub fn classCode(self: DeviceDescriptor) u8 {
        return self.descriptor.bDeviceClass;
    }

    pub fn subClassCode(self: DeviceDescriptor) u8 {
        return self.descriptor.bDeviceSubClass;
    }

    pub fn vendorId(self: DeviceDescriptor) u16 {
        return self.descriptor.idVendor;
    }

    pub fn productId(self: DeviceDescriptor) u16 {
        return self.descriptor.idProduct;
    }
};

pub const Device = struct {
    ctx: *c.libusb_context,
    device: *c.libusb_device,

    pub fn deinit(self: Device) void {
        _ = c.libusb_unref_device(self.device);
    }

    pub fn configDescriptor(self: Device, config_index: u8) Error!ConfigDescriptor {
        var descriptor: ?*c.libusb_config_descriptor = null;

        try failable(c.libusb_get_config_descriptor(
            self.device,
            config_index,
            &descriptor,
        ));

        return ConfigDescriptor{ .descriptor = descriptor.? };
    }

    pub fn deviceDescriptor(self: Device) Error!DeviceDescriptor {
        var descriptor: c.libusb_device_descriptor = undefined;

        try failable(c.libusb_get_device_descriptor(
            self.device,
            &descriptor,
        ));

        return DeviceDescriptor{ .descriptor = descriptor };
    }

    pub fn open(self: Device) Error!DeviceHandle {
        var handle: ?*c.libusb_device_handle = null;
        try failable(c.libusb_open(self.device, &handle));

        return DeviceHandle.fromLibusb(self.ctx, handle.?);
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
        return Device.fromLibusb(self.ctx, c.libusb_get_device(self.handle).?);
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
            self.handle,
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

    fn fromLibusb(ctx: *c.libusb_context, handle: *c.libusb_device_handle) DeviceHandle {
        return DeviceHandle{
            .ctx = ctx,
            .handle = handle,
            .interfaces = 0,
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
