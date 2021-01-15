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

    var endpoint_in: u8 = 0;
    var endpoint_out: u8 = 0;

    var i = config.interfaces();
    while (i.next()) |iface| {
        var ii = iface.descriptors();
        while (ii.next()) |descriptor| {
            var iii = descriptor.endpointDescriptors();
            while (iii.next()) |endpoint| {
                switch (endpoint.direction()) {
                    usb.Direction.In => {
                        endpoint_in = endpoint.address();
                    },
                    usb.Direction.Out => {
                        endpoint_out = endpoint.address();
                    },
                }
            }
        }
    }

    print("in: {}\n", .{endpoint_in});
    print("out: {}\n", .{endpoint_out});

    print("{}\n", .{config});
}
