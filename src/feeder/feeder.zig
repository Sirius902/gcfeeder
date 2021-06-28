const std = @import("std");
const ess = @import("ess/ess.zig");
const usb = @import("zusb");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const main_stick = @import("adapter.zig").Calibration.main_stick;
const c_stick = @import("adapter.zig").Calibration.c_stick;
const trigger_range = @import("adapter.zig").Calibration.trigger_range;

pub const Feeder = struct {
    pub const Error = Adapter.Error || vjoy.Device.Error;

    adapter: Adapter,
    device: vjoy.Device,

    pub fn init(ctx: *usb.Context) Error!Feeder {
        const adapter = try Adapter.init(ctx);
        errdefer adapter.deinit();

        const device = try vjoy.Device.init(1);

        return Feeder{ .adapter = adapter, .device = device };
    }

    pub fn deinit(self: Feeder) void {
        self.adapter.deinit();
        self.device.deinit();
    }

    pub fn feed(self: *Feeder, ess_adapter: bool) Error!?Input {
        const inputs = try self.adapter.readInputs();

        if (inputs[0]) |input| {
            if (ess_adapter) {
                try self.device.update(toVJoy(ess.map(input)));
            } else {
                try self.device.update(toVJoy(input));
            }

            return input;
        } else {
            return null;
        }
    }

    fn toVJoy(input: Input) vjoy.JoystickPosition {
        const windows_max = 32767;

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

        pos.wAxisX = @floatToInt(c_long, std.math.ceil(main_stick.normalize(input.stick_x) * windows_max));
        pos.wAxisY = @floatToInt(c_long, std.math.ceil((1.0 - main_stick.normalize(input.stick_y)) * windows_max));

        pos.wAxisXRot = @floatToInt(c_long, std.math.ceil(c_stick.normalize(input.substick_x) * windows_max));
        pos.wAxisYRot = @floatToInt(c_long, std.math.ceil((1.0 - c_stick.normalize(input.substick_y)) * windows_max));

        pos.wAxisZ = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_left) * windows_max));
        pos.wAxisZRot = @floatToInt(c_long, std.math.ceil(trigger_range.normalize(input.trigger_right) * windows_max));

        return pos;
    }
};
