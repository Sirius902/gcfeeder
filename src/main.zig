const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");
const usb = @import("usb.zig");

const gc_vid = 0x057E;
const gc_pid = 0x0337;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var handle = try ctx.openDeviceWithVidPid(gc_vid, gc_pid);
    defer handle.deinit();

    try handle.claimInterface(0);

    const device = handle.device();
    defer device.deinit();

    const config = try device.configDescriptor(0);
    defer config.deinit();

    print("{}\n", .{config});
}
