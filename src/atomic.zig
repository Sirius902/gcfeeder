const std = @import("std");

pub const Bool = struct {
    flag: std.atomic.Int(u1),

    pub fn init(init_val: bool) Bool {
        return Bool{ .flag = std.atomic.Int(u1).init(@boolToInt(init_val)) };
    }

    pub fn get(self: *Bool) bool {
        return self.flag.get() != 0;
    }

    pub fn set(self: *Bool, new_value: bool) void {
        _ = self.xchg(new_value);
    }

    pub fn xchg(self: *Bool, new_value: bool) bool {
        return self.flag.xchg(@boolToInt(new_value)) != 0;
    }
};
