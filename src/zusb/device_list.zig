const c = @import("c.zig");
const std = @import("std");
const Context = @import("context.zig").Context;
const fromLibusb = @import("constructor.zig").fromLibusb;

usingnamespace @import("error.zig");

pub const Devices = struct {
    ctx: *Context,
    devices: []?*c.libusb_device,
    i: usize,

    pub fn next(self: *Devices) ?Device {
        if (self.i < self.devices.len) {
            defer self.i += 1;
            return fromLibusb(self.ctx.ctx, self.devices[self.i].?);
        } else {
            return null;
        }
    }
};

pub const DeviceList = struct {
    ctx: *Context,
    list: [*c]?*c.libusb_device,
    len: usize,

    pub fn init(ctx: *Context) Error!DeviceList {
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
