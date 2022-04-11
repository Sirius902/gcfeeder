const std = @import("std");
const vjoy = @import("vjoy.zig");
const vigem = @import("vigem.zig");
const Allocator = std.mem.Allocator;
const Adapter = @import("../adapter.zig").Adapter;
const Input = @import("../adapter.zig").Input;
const Rumble = @import("../adapter.zig").Rumble;
const stick_range = @import("../adapter.zig").Calibration.stick_range;
const trigger_range = @import("../adapter.zig").Calibration.trigger_range;

const windows_max = 32767;

pub const VJoyBridge = struct {
    device: vjoy.Device,
    receiver: *vjoy.FFBReceiver,
    last_timestamp: ?i64 = null,

    pub const Error = vjoy.Device.Error || Allocator.Error;
    pub const driver_name = "VJoy";

    pub fn init(allocator: Allocator) Error!VJoyBridge {
        const device = try vjoy.Device.init(1);
        const receiver = try vjoy.FFBReceiver.init(allocator);
        return VJoyBridge{ .device = device, .receiver = receiver };
    }

    pub fn deinit(self: VJoyBridge) void {
        self.device.deinit();
        self.receiver.deinit();
    }

    pub fn feed(self: VJoyBridge, input: Input) Error!void {
        try self.device.update(toVJoy(input));
    }

    pub fn pollRumble(self: *VJoyBridge) ?Rumble {
        var rumble: ?Rumble = null;

        if (self.receiver.get()) |packet| {
            if (packet.device_id == 1) {
                rumble = switch (packet.effect.operation) {
                    .Stop => .Off,
                    else => .On,
                };

                if (self.last_timestamp) |last| {
                    if (packet.timestamp_ms - last < 2) {
                        rumble = .Off;
                    }
                }

                self.last_timestamp = packet.timestamp_ms;
            }
        }

        return rumble;
    }

    fn toVJoy(input: Input) vjoy.JoystickPosition {
        var pos = std.mem.zeroes(vjoy.JoystickPosition);

        var buttons = std.PackedIntArray(u1, 12).init([_]u1{
            @boolToInt(input.button_a),
            @boolToInt(input.button_b),
            @boolToInt(input.button_x),
            @boolToInt(input.button_y),
            @boolToInt(input.button_z),
            @boolToInt(input.button_r),
            @boolToInt(input.button_l),
            @boolToInt(input.button_start),
            @boolToInt(input.button_up),
            @boolToInt(input.button_down),
            @boolToInt(input.button_left),
            @boolToInt(input.button_right),
        });

        pos.lButtons = buttons.sliceCast(u12).get(0);

        pos.wAxisX = @floatToInt(c_long, std.math.ceil(stick_range.normalize(input.stick_x) * windows_max));
        pos.wAxisY = @floatToInt(c_long, std.math.ceil((1.0 - stick_range.normalize(input.stick_y)) * windows_max));

        pos.wAxisXRot = @floatToInt(c_long, std.math.ceil(stick_range.normalize(input.substick_x) * windows_max));
        pos.wAxisYRot = @floatToInt(c_long, std.math.ceil((1.0 - stick_range.normalize(input.substick_y)) * windows_max));

        pos.wAxisZ = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_left) * windows_max));
        pos.wAxisZRot = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_right) * windows_max));

        return pos;
    }
};

pub const ViGEmBridge = struct {
    device: vigem.Device,

    pub const Error = vigem.Device.Error || Allocator.Error;
    pub const driver_name = "ViGEm";

    pub fn init(allocator: Allocator) Error!ViGEmBridge {
        _ = allocator;
        const device = try vigem.Device.init();
        return ViGEmBridge{ .device = device };
    }

    pub fn deinit(self: ViGEmBridge) void {
        self.device.deinit();
    }

    pub fn feed(self: ViGEmBridge, input: Input) Error!void {
        try self.device.update(toDS4(input));
    }

    pub fn pollRumble(self: *ViGEmBridge) ?Rumble {
        _ = self;
        return Rumble.Off;
    }

    fn toDS4(input: Input) vigem.DS4Report {
        var pos = std.mem.zeroes(vigem.DS4Report);

        const hat_bits = hatBits(input);
        var buttons = std.PackedIntArray(u1, 14).init([_]u1{
            @truncate(u1, hat_bits >> 0),
            @truncate(u1, hat_bits >> 1),
            @truncate(u1, hat_bits >> 2),
            @truncate(u1, hat_bits >> 3),
            @boolToInt(input.button_x),
            @boolToInt(input.button_a),
            @boolToInt(input.button_b),
            @boolToInt(input.button_y),
            0,
            @boolToInt(input.button_z),
            @boolToInt(input.button_l),
            @boolToInt(input.button_r),
            0,
            @boolToInt(input.button_start),
        });

        pos.wButtons = buttons.sliceCast(u14).get(0);

        pos.bThumbLX = input.stick_x;
        pos.bThumbLY = ~input.stick_y;

        pos.bThumbRX = input.substick_x;
        pos.bThumbRY = ~input.substick_y;

        pos.bTriggerL = input.trigger_left;
        pos.bTriggerR = input.trigger_right;

        return pos;
    }

    fn hatBits(input: Input) u4 {
        const up: u4 = @boolToInt(input.button_up);
        const left: u4 = @boolToInt(input.button_left);
        const right: u4 = @boolToInt(input.button_right);
        const down: u4 = @boolToInt(input.button_down);
        const encoded = (up << 3) | (left << 2) | (right << 1) | down;

        return switch (encoded) {
            0b0000, 0b1111, 0b1001, 0b0110 => 0b1000,
            0b1000, 0b1110 => 0,
            0b1010 => 1,
            0b0010, 0b1011 => 2,
            0b0011 => 3,
            0b0001, 0b0111 => 4,
            0b0101 => 5,
            0b0100, 0b1101 => 6,
            0b1100 => 7,
        };
    }
};
