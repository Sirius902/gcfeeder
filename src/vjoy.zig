const std = @import("std");
const c = @import("c.zig");

pub const Error = error{
    Acquire,
    Update,
};

pub const JoystickPosition = c.JOYSTICK_POSITION_V2;

pub const Stat = enum {
    Own,
    Free,
    Busy,
    Miss,
    Unknown,
};

pub const Device = struct {
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
