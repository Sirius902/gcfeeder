const std = @import("std");
const time = std.time;
const print = std.debug.print;

const usb = @cImport({
    @cInclude("libusb-1.0/libusb.h");
});

pub fn main() void {
    var ctx: ?*usb.libusb_context = null;
    _ = usb.libusb_init(&ctx);
    defer usb.libusb_exit(ctx);

    print("Hello World!\n", .{});
}
