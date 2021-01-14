const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");

pub fn main() void {
    var ctx: ?*c.libusb_context = null;
    _ = c.libusb_init(&ctx);
    defer c.libusb_exit(ctx);

    print("Hello World!\n", .{});
    print("libusb_context: {p}\n", .{ctx});
    print("vJoyEnabled: {}\n", .{c.vJoyEnabled()});

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.InitWindow(screen_width, screen_height, "raylib window");

    std.time.sleep(std.time.ns_per_s);

    c.CloseWindow();
}
