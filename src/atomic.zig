const std = @import("std");

pub const Bool = struct {
    flag: std.atomic.Int(u1),

    pub fn init(init_val: bool) @This() {
        return @This(){ .flag = std.atomic.Int(u1).init(@boolToInt(init_val)) };
    }

    pub fn get(self: *@This()) bool {
        return self.flag.get() != 0;
    }

    pub fn set(self: *@This(), new_value: bool) void {
        _ = self.xchg(new_value);
    }

    pub fn xchg(self: *@This(), new_value: bool) bool {
        return self.flag.xchg(@boolToInt(new_value)) != 0;
    }
};
