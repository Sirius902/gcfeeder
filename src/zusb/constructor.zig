const c = @import("c.zig");
const std = @import("std");
const Device = @import("device.zig").Device;
const DeviceHandle = @import("device_handle.zig").DeviceHandle;

pub fn fromLibusb(comptime T: type, args: anytype) T {
    switch (T) {
        Device => {
            _ = c.libusb_ref_device(args.@"1");
            return .{
                .ctx = args.@"0",
                .raw = args.@"1",
            };
        },
        DeviceHandle => {
            return .{
                .ctx = args.@"0",
                .raw = args.@"1",
                .interfaces = 0,
            };
        },
        else => {
            @compileError("Unsupported type");
        },
    }
}
