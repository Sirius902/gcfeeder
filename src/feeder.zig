const std = @import("std");
const ad = @import("adapter.zig");
const usb = @import("usb.zig");
const vjoy = @import("vjoy.zig");

pub const Error = ad.Error || vjoy.Error;

pub const Feeder = struct {
    adapter: ad.Adapter,
    device: vjoy.Device,

    pub fn init(ctx: *usb.Context) Error!Feeder {
        const adapter = try ad.Adapter.init(ctx);
        errdefer adapter.deinit();

        const device = try vjoy.Device.init(1);

        return Feeder{ .adapter = adapter, .device = device };
    }

    pub fn deinit(self: Feeder) void {
        self.adapter.deinit();
        self.device.deinit();
    }

    pub fn feed(self: *Feeder) ?ad.Input {
        const inputs = self.adapter.readInputs() catch null;
        const input = if (inputs) |ins| ins[0] else null;

        if (input) |in| {
            _ = self.device.update(toVJoy(in)) catch {};

            return in;
        } else {
            return null;
        }
    }

    fn toVJoy(input: ad.Input) vjoy.JoystickPosition {
        const MULT = 0x7F;

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

        pos.wAxisX = @as(c_long, input.stick_x) * MULT;
        pos.wAxisY = @as(c_long, ~input.stick_y) * MULT;

        pos.wAxisXRot = @as(c_long, input.substick_x) * MULT;
        pos.wAxisYRot = @as(c_long, ~input.substick_y) * MULT;

        pos.wAxisZ = @as(c_long, input.trigger_left) * MULT;
        pos.wAxisZRot = @as(c_long, input.trigger_right) * MULT;

        return pos;
    }
};
