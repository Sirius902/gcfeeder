const std = @import("std");
const usb = @import("zusb");

const payload_len = 37;

pub const Adapter = struct {
    pub const vid = 0x057E;
    pub const pid = 0x0337;

    pub const Error = error{
        Unplugged,
        Payload,
    } || usb.Error;

    const allowed_timeout_ms = 16;

    const Endpoints = struct {
        in: u8,
        out: u8,
    };

    handle: usb.DeviceHandle,
    endpoints: Endpoints,
    calibrations: [4]?Calibration,

    pub fn init(ctx: *usb.Context) Error!Adapter {
        var handle = (try ctx.openDeviceWithVidPid(vid, pid)) orelse {
            return Error.Unplugged;
        };

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
            .calibrations = [_]?Calibration{null} ** 4,
        };
    }

    pub fn deinit(self: Adapter) void {
        self.handle.deinit();
    }

    pub fn readInputs(self: *Adapter) Error![4]?Input {
        const payload = try self.readPayload();
        var inputs = [_]?Input{null} ** 4;

        for (Port.all()) |port| {
            const chan = port.channel();
            // type is 0 if no controller is plugged, 1 if wired, and 2 if wireless
            const controller_type = payload[1 + (9 * chan)] >> 4;
            const connected = controller_type != 0;

            if (!connected) {
                self.calibrations[chan] = null;
                continue;
            }

            const raw = Input.fromPayload(&payload, port);

            if (self.calibrations[chan]) |calibration| {
                inputs[chan] = calibration.correct(raw);
            } else {
                const calibration = Calibration.init(raw);
                inputs[chan] = calibration.correct(raw);
                self.calibrations[chan] = calibration;
            }
        }

        return inputs;
    }

    pub fn setRumble(self: Adapter, states: [4]Rumble) Error!void {
        const payload = [5]u8{
            0x11,
            states[0].toByte(),
            states[1].toByte(),
            states[2].toByte(),
            states[3].toByte(),
        };

        _ = try self.handle.writeInterrupt(
            self.endpoints.out,
            &payload,
            allowed_timeout_ms,
        );
    }

    pub fn resetRumble(self: Adapter) Error!void {
        try self.setRumble(.{ .Off, .Off, .Off, .Off });
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

        if (bytes_read != payload_len or payload[0] != usb.dt_hid) {
            return Error.Payload;
        }

        return payload;
    }
};

pub const Rumble = enum {
    On,
    Off,

    pub fn toByte(self: Rumble) u8 {
        return switch (self) {
            .Off => 0,
            .On => 1,
        };
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

const Vec2 = struct {
    x: u8,
    y: u8,
};

const StickRange = struct {
    center: u8,
    radius: u8,

    pub fn restrict(self: StickRange, x: i10, y: i10) Vec2 {
        const center = @as(i10, self.center);
        const radius = @as(i10, self.radius);

        const xx = std.math.clamp(x, center - radius, center + radius);
        const yy = std.math.clamp(y, center - radius, center + radius);

        return Vec2{
            .x = @intCast(u8, xx),
            .y = @intCast(u8, yy),
        };
    }

    pub fn normalize(self: StickRange, axis: u8) f64 {
        return @intToFloat(f64, axis) / @intToFloat(f64, self.center + self.radius);
    }
};

const AnalogRange = struct {
    min: u8,
    max: u8,

    pub fn restrict(self: AnalogRange, n: i10) u8 {
        const nn = std.math.clamp(n, @as(i10, self.min), @as(i10, self.max));

        return @intCast(u8, nn);
    }

    pub fn normalize(self: AnalogRange, axis: u8) f64 {
        return @intToFloat(f64, axis + self.min) / @intToFloat(f64, self.max - self.min);
    }
};

pub const Calibration = struct {
    pub const stick_range = StickRange{ .center = 0x80, .radius = 0x7F };
    pub const trigger_range = AnalogRange{ .min = 0x00, .max = 0xFF };

    stick_x: i10,
    stick_y: i10,
    substick_x: i10,
    substick_y: i10,
    trigger_left: i10,
    trigger_right: i10,

    pub fn init(initial: Input) Calibration {
        return Calibration{
            .stick_x = @as(i10, stick_range.center) - initial.stick_x,
            .stick_y = @as(i10, stick_range.center) - initial.stick_y,
            .substick_x = @as(i10, stick_range.center) - initial.substick_x,
            .substick_y = @as(i10, stick_range.center) - initial.substick_y,
            .trigger_left = @as(i10, trigger_range.min) - initial.trigger_left,
            .trigger_right = @as(i10, trigger_range.min) - initial.trigger_right,
        };
    }

    pub fn correct(self: Calibration, input: Input) Input {
        var in = input;

        const stick = stick_range.restrict(
            @as(i10, in.stick_x) + self.stick_x,
            @as(i10, in.stick_y) + self.stick_y,
        );

        const substick = stick_range.restrict(
            @as(i10, in.substick_x) + self.substick_x,
            @as(i10, in.substick_y) + self.substick_y,
        );

        in.stick_x = stick.x;
        in.stick_y = stick.y;
        in.substick_x = substick.x;
        in.substick_y = substick.y;
        in.trigger_left = trigger_range.restrict(@as(i10, in.trigger_left) + self.trigger_left);
        in.trigger_right = trigger_range.restrict(@as(i10, in.trigger_right) + self.trigger_right);

        return in;
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

    pub fn serialize(self: Input, buffer: *[@sizeOf(Input)]u8) void {
        inline for (@typeInfo(Input).Struct.fields) |field, i| {
            buffer[i] = switch (field.field_type) {
                u8 => @field(self, field.name),
                bool => @boolToInt(@field(self, field.name)),
                else => @compileError("Unsupported type"),
            };
        }
    }

    pub fn deserialize(buffer: *const [@sizeOf(Input)]u8) Input {
        var input: Input = undefined;

        inline for (@typeInfo(Input).Struct.fields) |field, i| {
            @field(input, field.name) = switch (field.field_type) {
                u8 => buffer[i],
                bool => buffer[i] != 0,
                else => @compileError("Unsupported type"),
            };
        }

        return input;
    }

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
