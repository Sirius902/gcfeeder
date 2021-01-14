const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @cImport({
    @cInclude("libusb-1.0/libusb.h");
    @cInclude("vjoyinterface.h");
});

pub fn main() void {
    var ctx: ?*c.libusb_context = null;
    _ = c.libusb_init(&ctx);
    defer c.libusb_exit(ctx);

    print("Hello World!\n", .{});
    print("libusb_context: {p}!\n", .{ctx});
    print("vJoyEnabled: {p}!\n", .{c.vJoyEnabled()});
}
