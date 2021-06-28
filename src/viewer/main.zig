const std = @import("std");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;

pub fn main() !void {
    std.log.info("{}\n", .{@sizeOf(Input)});
}
