const std = @import("std");
const math = std.math;
const Input = @import("../adapter.zig").Input;
const main_stick = @import("../adapter.zig").Calibration.main_stick;
const vc_map: []const u8 = @embedFile("map/oot-vc.bin");

pub const Quadrant = enum {
    One,
    Two,
    Three,
    Four,

    pub fn of(coords: *const [2]u8) Quadrant {
        if (coords[0] >= main_stick.center) {
            return if (coords[1] >= main_stick.center) .One else .Four;
        } else {
            return if (coords[1] >= main_stick.center) .Two else .Three;
        }
    }
};

/// Returns GC coordinates normalized to quadrant one.
///
/// Note: x and y should be nonzero. As a workaround, zero coordinates are mapped to 1.
fn normalize(coords: *[2]u8) Quadrant {
    const original = Quadrant.of(coords);
    const x = @as(i10, coords[0]);
    const y = @as(i10, coords[1]);

    const xx = switch (original) {
        .One, .Four => x - main_stick.center,
        .Two, .Three => math.min(main_stick.center - x, main_stick.radius),
    };

    const yy = switch (original) {
        .One, .Two => y - main_stick.center,
        .Three, .Four => math.min(main_stick.center - y, main_stick.radius),
    };

    coords[0] = @intCast(u8, math.clamp(xx, math.minInt(u8), math.maxInt(u8)));
    coords[1] = @intCast(u8, math.clamp(yy, math.minInt(u8), math.maxInt(u8)));
    return original;
}

/// Denormalizes GC coordinates from quadrant one back to their original quadrant.
fn denormalize(coords: *[2]u8, original: Quadrant) void {
    const x = @as(i10, coords[0]);
    const y = @as(i10, coords[1]);

    const xx = switch (original) {
        .One, .Four => x + main_stick.center,
        .Two, .Three => main_stick.center - x,
    };

    const yy = switch (original) {
        .One, .Two => y + main_stick.center,
        .Three, .Four => main_stick.center - y,
    };

    coords[0] = @intCast(u8, math.clamp(xx, math.minInt(u8), math.maxInt(u8)));
    coords[1] = @intCast(u8, math.clamp(yy, math.minInt(u8), math.maxInt(u8)));
}

/// Maps first quadrant normalized GC coordinates to fit to the shape of the N64 controller.
/// Uses math from https://github.com/Skuzee/ESS-Adapter.
///
/// Precondition: `0 <= y <= x <= 127`
fn gcToN64(coords: *[2]u8) void {
    const x = @intToFloat(f64, coords[0]);
    const y = @intToFloat(f64, coords[1]);

    const scale = math.pow(f64, (5.0 * x + 2.0 * y) / 525.0, 2.0) * (7.0 * y / 525.0) * (70.0 / 75.0 - 80.0 / 105.0) + 80.0 / 105.0;

    coords[0] = math.min(@floatToInt(u8, @ceil(x * scale)), 127);
    coords[1] = math.min(@floatToInt(u8, @ceil(y * scale)), 127);
}

pub fn map(input: Input) Input {
    const swap = input.stick_y > input.stick_x;
    var coords = [_]u8{ input.stick_x, input.stick_y };

    const q = normalize(&coords);
    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    gcToN64(&coords);

    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    const map_index = 2 * ((@as(usize, coords[1]) * 128) + coords[0]);
    coords[0] = vc_map[map_index];
    coords[1] = vc_map[map_index + 1];

    denormalize(&coords, q);

    var mapped = input;
    mapped.stick_x = coords[0];
    mapped.stick_y = coords[1];
    return mapped;
}
