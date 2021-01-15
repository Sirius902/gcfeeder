const std = @import("std");
const c = @import("c.zig");
const usb = @import("usb.zig");

const payload_len = 37;

pub const Error = error{Payload} || usb.Error;

pub const Adapter = struct {
    const gc_vid = 0x057E;
    const gc_pid = 0x0337;

    const allowed_timeout_ms = 16;

    const Endpoints = struct {
        in: u8,
        out: u8,
    };

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

    pub fn readInputs(self: Adapter) Error![4]?Input {
        const payload = try self.readPayload();
        var inputs = [_]?Input{null} ** 4;

        for (Port.all()) |port| {
            const chan = port.channel();
            // type is 0 if no controller is plugged, 1 if wired, and 2 if wireless
            const controller_type = payload[1 + (9 * chan)] >> 4;
            const connected = controller_type != 0;

            if (!connected) {
                // self.calibrations[chan] = null;
                continue;
            }

            inputs[chan] = Input.fromPayload(&payload, port);
        }

        return inputs;
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

pub const Port = enum {
    One,
    Two,
    Three,
    Four,

    pub fn channel(self: Port) usize {
        return switch (self) {
            .One => 0,
            .Two => 1,
            .Three => 2,
            .Four => 3,
        };
    }

    pub fn all() [4]Port {
        return [_]Port{ .One, .Two, .Three, .Four };
    }
};

pub const Input = struct {
    button_a: bool,
    button_b: bool,
    button_x: bool,
    button_y: bool,

    button_left: bool,
    button_right: bool,
    button_down: bool,
    button_up: bool,

    button_start: bool,
    button_z: bool,
    button_r: bool,
    button_l: bool,

    stick_x: u8,
    stick_y: u8,
    substick_x: u8,
    substick_y: u8,
    trigger_left: u8,
    trigger_right: u8,

    fn fromPayload(payload: *const [payload_len]u8, port: Port) Input {
        const chan = port.channel();
        const b1 = payload[1 + (9 * chan) + 1];
        const b2 = payload[1 + (9 * chan) + 2];

        return Input{
            .button_a = (b1 & (1 << 0)) != 0,
            .button_b = (b1 & (1 << 1)) != 0,
            .button_x = (b1 & (1 << 2)) != 0,
            .button_y = (b1 & (1 << 3)) != 0,

            .button_left = (b1 & (1 << 4)) != 0,
            .button_right = (b1 & (1 << 5)) != 0,
            .button_down = (b1 & (1 << 6)) != 0,
            .button_up = (b1 & (1 << 7)) != 0,

            .button_start = (b2 & (1 << 0)) != 0,
            .button_z = (b2 & (1 << 1)) != 0,
            .button_r = (b2 & (1 << 2)) != 0,
            .button_l = (b2 & (1 << 3)) != 0,

            .stick_x = payload[1 + (9 * chan) + 3],
            .stick_y = payload[1 + (9 * chan) + 4],
            .substick_x = payload[1 + (9 * chan) + 5],
            .substick_y = payload[1 + (9 * chan) + 6],
            .trigger_left = payload[1 + (9 * chan) + 7],
            .trigger_right = payload[1 + (9 * chan) + 8],
        };
    }
};
