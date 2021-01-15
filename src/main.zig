const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");
const usb = @import("usb.zig");
const Adapter = @import("adapter.zig").Adapter;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var adapter = try Adapter.init(&ctx);
    defer adapter.deinit();

    _ = try adapter.handle.writeInterrupt(adapter.endpoints.out, &[_]u8{ 0x11, 0x01, 0x01, 0x01, 0x01 }, 16);

    std.time.sleep(std.time.ns_per_s);

    _ = try adapter.handle.writeInterrupt(adapter.endpoints.out, &[_]u8{ 0x11, 0x00, 0x00, 0x00, 0x00 }, 16);
}
