const std = @import("std");
const c = @import("c.zig");
const usb = @import("usb.zig");

pub const Error = error{Payload} || usb.Error;

pub const Adapter = struct {
    const gc_vid = 0x057E;
    const gc_pid = 0x0337;

    const allowed_timeout_ms = 16;

    const Endpoints = struct {
        in: u8,
        out: u8,
    };

    pub const payload_len = 37;

    handle: usb.DeviceHandle,
    endpoints: Endpoints,

    pub fn init(ctx: *usb.Context) Error!Adapter {
        var handle = try ctx.openDeviceWithVidPid(gc_vid, gc_pid);

        try handle.claimInterface(0);

        const endpoints = try findEndpoints(handle);

        // From Dolphin:
        // This call makes Nyko-brand (and perhaps other) adapters work.
        // However it returns LIBUSB_ERROR_PIPE with Mayflash adapters.
        _ = handle.writeControl(0x21, 11, 0x0001, 0, &[_]u8{}, std.time.ms_per_s) catch {};

        // Not sure what this does but Dolphin does it
        _ = handle.writeInterrupt(endpoints.out, &[_]u8{0x13}, allowed_timeout_ms) catch {};

        return Adapter{
            .handle = handle,
            .endpoints = endpoints,
        };
    }

    pub fn deinit(self: Adapter) void {
        self.handle.deinit();
    }

    fn findEndpoints(handle: usb.DeviceHandle) Error!Endpoints {
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

    fn readPayload(self: Adapter) Error![payload_len]u8 {
        var payload: [payload_len]u8 = undefined;

        const bytes_read = try self.handle.readInterrupt(
            self.endpoints.in,
            &payload,
            allowed_timeout_ms,
        );

        if (bytes_read != payload_len or payload[0] != c.LIBUSB_DT_HID) {
            return Error.Payload;
        }

        return payload;
    }
};
