const std = @import("std");
const c = @import("c.zig");
const atomic = std.atomic;
const Allocator = std.mem.Allocator;
const LinearFifo = std.fifo.LinearFifo;
const Mutex = std.Thread.Mutex;

pub const JoystickPosition = c.JOYSTICK_POSITION_V2;

pub const Stat = enum {
    Own,
    Free,
    Busy,
    Miss,
    Unknown,
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

pub const FFBReciever = struct {
    const Fifo = LinearFifo(FFBPacket, .{ .Static = 10 });

    allocator: *Allocator,
    mutex: Mutex,
    queue: Fifo,

    pub fn init(allocator: *Allocator) Allocator.Error!*FFBReciever {
        var self = try allocator.create(FFBReciever);
        self.* = FFBReciever{
            .allocator = allocator,
            .mutex = Mutex{},
            .queue = Fifo.init(),
        };

        c.FfbRegisterGenCB(ffbCallback, self);

        return self;
    }

    pub fn deinit(self: *FFBReciever) void {
        self.allocator.destroy(self);
    }

    pub fn get(self: *FFBReciever) ?FFBPacket {
        const held = self.mutex.acquire();
        defer held.release();

        return self.queue.readItem();
    }

    fn put(self: *FFBReciever, packet: FFBPacket) void {
        const held = self.mutex.acquire();
        defer held.release();

        self.queue.ensureUnusedCapacity(1) catch {
            self.queue.discard(1);
        };

        self.queue.writeItemAssumeCapacity(packet);
    }

    export fn ffbCallback(data: ?*c_void, userdata: ?*c_void) void {
        const ffb_data = @intToPtr(*c.FFB_DATA, @ptrToInt(data.?));
        const self = @intToPtr(*FFBReciever, @ptrToInt(userdata.?));
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
