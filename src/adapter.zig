const std = @import("std");
const usb = @import("usb.zig");

pub const Adapter = struct {
    const gc_vid = 0x057E;
    const gc_pid = 0x0337;

    const allowed_timeout = 16;

    const Endpoints = struct {
        in: u8,
        out: u8,
    };

    handle: usb.DeviceHandle,
    endpoints: Endpoints,

    pub fn init(ctx: *usb.Context) usb.Error!Adapter {
        var handle = try ctx.openDeviceWithVidPid(gc_vid, gc_pid);

        try handle.claimInterface(0);

        const endpoints = try findEndpoints(handle);

        // From Dolphin:
        // This call makes Nyko-brand (and perhaps other) adapters work.
        // However it returns LIBUSB_ERROR_PIPE with Mayflash adapters.
        _ = handle.writeControl(0x21, 11, 0x0001, 0, &[_]u8{}, std.time.ms_per_s) catch {};

        // Not sure what this does but Dolphin does it
        _ = handle.writeInterrupt(endpoints.out, &[_]u8{0x13}, allowed_timeout) catch {};

        return Adapter{
            .handle = handle,
            .endpoints = endpoints,
        };
    }

    pub fn deinit(self: Adapter) void {
        self.handle.deinit();
    }

    fn findEndpoints(handle: usb.DeviceHandle) usb.Error!Endpoints {
        const device = handle.device();
        defer device.deinit();

        const config = try device.configDescriptor(0);
        defer config.deinit();

        var in: u8 = 0;
        var out: u8 = 0;

        var interfaces = config.interfaces();
        while (interfaces.next()) |iface| {
            var descriptors = iface.descriptors();
            while (descriptors.next()) |descriptor| {
                var endpoints = descriptor.endpointDescriptors();
                while (endpoints.next()) |endpoint| {
                    switch (endpoint.direction()) {
                        usb.Direction.In => {
                            in = endpoint.address();
                        },
                        usb.Direction.Out => {
                            out = endpoint.address();
                        },
                    }
                }
            }
        }

        return Endpoints{ .in = in, .out = out };
    }
};
