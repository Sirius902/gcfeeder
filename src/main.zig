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

    print("{}\n", .{adapter.readInputs()});
}
