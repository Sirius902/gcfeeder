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

pub const FFBPacket = union(enum) {
    pub const EffectOperation = struct {
        device_id: u8,
        operation: c.FFB_EFF_OP,
        timestamp_ms: i64,
    };

    pub const EffectReport = struct {
        device_id: u8,
        report: c.FFB_EFF_REPORT,
    };

    effect_operation: EffectOperation,
    effect_report: EffectReport
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
        var id: c_int = undefined;
        var ffb_type: c.FFBPType = undefined;

        if (c.Ffb_h_DeviceID(ffb_data, &id) == c.ERROR_SEVERITY_SUCCESS and c.Ffb_h_Type(ffb_data, &ffb_type) == c.ERROR_SEVERITY_SUCCESS) {
            switch (ffb_type) {
                .PT_EFOPREP => {
                    var operation: c.FFB_EFF_OP = undefined;

                    if (c.Ffb_h_EffOp(ffb_data, &operation) == c.ERROR_SEVERITY_SUCCESS) {
                        self.put(FFBPacket{
                            .effect_operation = .{
                                .device_id = std.math.cast(u8, id) catch unreachable,
                                .operation = operation,
                                .timestamp_ms = std.time.milliTimestamp(),
                            },
                        });
                    }
                },
                .PT_EFFREP => {
                    var report: c.FFB_EFF_REPORT = undefined;

                    if (c.Ffb_h_Eff_Report(ffb_data, &report) == c.ERROR_SEVERITY_SUCCESS) {
                        self.put(
                            FFBPacket{
                                .effect_report = .{
                                    .device_id = std.math.cast(u8, id) catch unreachable,
                                    .report = report,
                                },
                            },
                        );
                    }
                },
                else => {},
            }
        }
    }
};
