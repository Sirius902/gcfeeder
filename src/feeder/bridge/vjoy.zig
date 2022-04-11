const std = @import("std");
pub const c = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
    @cInclude("vjoyinterface.h");
});
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const LinearFifo = std.fifo.LinearFifo;
const Mutex = std.Thread.Mutex;
const Adapter = @import("../adapter.zig").Adapter;
const Input = @import("../adapter.zig").Input;
const Rumble = @import("../adapter.zig").Rumble;
const stick_range = @import("../adapter.zig").Calibration.stick_range;
const trigger_range = @import("../adapter.zig").Calibration.trigger_range;

pub const JoystickPosition = c.JOYSTICK_POSITION_V2;

pub const Stat = enum {
    Own,
    Free,
    Busy,
    Miss,
    Unknown,
};

pub const Bridge = struct {
    device: Device,
    receiver: *FFBReceiver,
    last_timestamp: ?i64 = null,

    pub const Error = Adapter.Error || Device.Error || Allocator.Error;

    pub fn init(allocator: Allocator) Error!Bridge {
        const device = try Device.init(1);
        const receiver = try FFBReceiver.init(allocator);
        return Bridge{ .device = device, .receiver = receiver };
    }

    pub fn deinit(self: Bridge) void {
        self.device.deinit();
        self.receiver.deinit();
    }

    pub fn feed(self: Bridge, input: Input) Error!void {
        try self.device.update(toVJoy(input));
    }

    pub fn pollRumble(self: *Bridge) ?Rumble {
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

    fn toVJoy(input: Input) JoystickPosition {
        const windows_max = 32767;

        var pos = std.mem.zeroes(JoystickPosition);

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

pub const Device = struct {
    pub const Error = error{
        Acquire,
        Update,
    };

    id: u8,

    pub fn init(id: u8) Error!Device {
        if (id < 1 or id > 15) @panic("Invalid device id");

        if (c.AcquireVJD(id) == c.TRUE) {
            return Device{ .id = id };
        } else {
            return Error.Acquire;
        }
    }

    pub fn deinit(self: Device) void {
        _ = c.RelinquishVJD(self.id);
    }

    pub fn update(self: Device, position: JoystickPosition) Error!void {
        var pos = position;
        pos.bDevice = self.id;

        if (c.UpdateVJD(self.id, &pos) == c.FALSE) {
            return Error.Update;
        }
    }

    pub fn status(self: Device) Stat {
        return switch (c.GetVJDStatus(self.id)) {
            c.VJD_STAT_OWN => Stat.Own,
            c.VJD_STAT_FREE => Stat.Free,
            c.VJD_STAT_BUSY => Stat.Busy,
            c.VJD_STAT_MISS => Stat.Miss,
            c.VJD_STAT_UNKN => Stat.Unknown,
            else => Stat.Unknown,
        };
    }

    pub fn buttonCount(self: Device) i32 {
        return c.GetVJDButtonNumber(self.id);
    }
};

pub fn driverEnabled() bool {
    return c.vJoyEnabled() == c.TRUE;
}

pub const Operation = enum {
    Start,
    Solo,
    Stop,
};

pub const EffectOperation = struct {
    block_index: u8,
    operation: Operation,
    loop_count: u8,

    fn fromVJoy(eff_op: c.FFB_EFF_OP) EffectOperation {
        return EffectOperation{
            .block_index = eff_op.EffectBlockIndex,
            .operation = switch (eff_op.EffectOp) {
                c.EFF_START => .Start,
                c.EFF_SOLO => .Solo,
                c.EFF_STOP => .Stop,
                else => unreachable,
            },
            .loop_count = eff_op.LoopCount,
        };
    }
};

pub const FFBPacket = struct {
    device_id: u8,
    effect: EffectOperation,
    timestamp_ms: i64,
};

pub const FFBReceiver = struct {
    const Fifo = LinearFifo(FFBPacket, .{ .Static = 10 });

    allocator: Allocator,
    mutex: Mutex,
    queue: Fifo,

    pub fn init(allocator: Allocator) Allocator.Error!*FFBReceiver {
        var self = try allocator.create(FFBReceiver);
        self.* = FFBReceiver{
            .allocator = allocator,
            .mutex = Mutex{},
            .queue = Fifo.init(),
        };

        c.FfbRegisterGenCB(ffbCallback, self);

        return self;
    }

    pub fn deinit(self: *FFBReceiver) void {
        self.allocator.destroy(self);
    }

    pub fn get(self: *FFBReceiver) ?FFBPacket {
        self.mutex.lock();
        defer self.mutex.unlock();

        return self.queue.readItem();
    }

    fn put(self: *FFBReceiver, packet: FFBPacket) void {
        self.mutex.lock();
        defer self.mutex.unlock();

        self.queue.ensureUnusedCapacity(1) catch {
            self.queue.discard(1);
        };

        self.queue.writeItemAssumeCapacity(packet);
    }

    export fn ffbCallback(data: ?*anyopaque, userdata: ?*anyopaque) void {
        const ffb_data = @intToPtr(*c.FFB_DATA, @ptrToInt(data.?));
        const self = @intToPtr(*FFBReceiver, @ptrToInt(userdata.?));
        var c_id: c_int = undefined;
        var ffb_type: c.FFBPType = undefined;

        if (c.Ffb_h_DeviceID(ffb_data, &c_id) == c.ERROR_SEVERITY_SUCCESS and c.Ffb_h_Type(ffb_data, &ffb_type) == c.ERROR_SEVERITY_SUCCESS) {
            const id = @intCast(u8, c_id);

            const timestamp = std.time.milliTimestamp();

            switch (ffb_type) {
                c.PT_EFOPREP => {
                    var operation: c.FFB_EFF_OP = undefined;

                    if (c.Ffb_h_EffOp(ffb_data, &operation) == c.ERROR_SEVERITY_SUCCESS) {
                        self.put(FFBPacket{
                            .device_id = id,
                            .effect = EffectOperation.fromVJoy(operation),
                            .timestamp_ms = timestamp,
                        });
                    }
                },
                else => {},
            }
        }
    }
};
