const std = @import("std");
const math = std.math;
const Tuple = std.meta.Tuple;
const Input = @import("../adapter.zig").Input;
const stick_range = @import("../adapter.zig").Calibration.stick_range;

pub const Mapping = enum {
    @"oot-vc",
    @"mm-vc",
    @"z64-gc",

    pub fn fileName(self: Mapping) []const u8 {
        return std.meta.tagName(self);
    }

    pub fn fromFileName(file_name: []const u8) ?Mapping {
        return std.meta.stringToEnum(Mapping, file_name);
    }

    pub fn normalizedMap(self: Mapping) NormalizedMap {
        inline for (comptime std.enums.values(Mapping)) |variant| {
            if (self == variant) {
                return NormalizedMap{ .table = @embedFile("map/" ++ variant.fileName() ++ ".bin") };
            }
        }

        unreachable;
    }

    pub fn applyScaling(self: Mapping, coords: *[2]u8) void {
        gcToN64(coords);
    }

    pub fn jsonStringify(
        value: Mapping,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        _ = options;
        try out_stream.writeByte('"');
        try out_stream.writeAll(std.meta.tagName(value));
        try out_stream.writeByte('"');
    }
};

/// Mapping of normalized quadrant one GC coordinates using a LUT.
/// Rows are y and columns are x.
pub const NormalizedMap = struct {
    table: *const [dim * dim * 2]u8,

    pub const dim = 128;

    /// Maps normalized GC coordinates.
    /// Should not be called on raw GC coordinates.
    pub fn map(self: NormalizedMap, coords: *[2]u8) void {
        const index = 2 * ((@as(usize, coords[1]) * dim) + coords[0]);
        coords[0] = self.table[index];
        coords[1] = self.table[index + 1];
    }
};

pub const Quadrant = enum {
    one,
    two,
    three,
    four,

    pub fn of(coords: *const [2]u8) Quadrant {
        if (coords[0] >= stick_range.center) {
            return if (coords[1] >= stick_range.center) .one else .four;
        } else {
            return if (coords[1] >= stick_range.center) .two else .three;
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
        .one, .four => x - stick_range.center,
        .two, .three => math.min(stick_range.center - x, stick_range.radius),
    };

    const yy = switch (original) {
        .one, .two => y - stick_range.center,
        .three, .four => math.min(stick_range.center - y, stick_range.radius),
    };

    coords[0] = math.lossyCast(u8, xx);
    coords[1] = math.lossyCast(u8, yy);
    return original;
}

/// Denormalizes GC coordinates from quadrant one back to their original quadrant.
fn denormalize(coords: *[2]u8, original: Quadrant) void {
    const x = @as(i10, coords[0]);
    const y = @as(i10, coords[1]);

    const xx = switch (original) {
        .one, .four => x + stick_range.center,
        .two, .three => stick_range.center - x,
    };

    const yy = switch (original) {
        .one, .two => y + stick_range.center,
        .three, .four => stick_range.center - y,
    };

    coords[0] = math.lossyCast(u8, xx);
    coords[1] = math.lossyCast(u8, yy);
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

pub fn map(mapping: Mapping, input: Input) Input {
    const swap = input.stick_y > input.stick_x;
    var coords = [_]u8{ input.stick_x, input.stick_y };

    const q = normalize(&coords);
    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    mapping.applyScaling(&coords);

    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    mapping.normalizedMap().map(&coords);

    denormalize(&coords, q);

    var mapped = input;
    mapped.stick_x = coords[0];
    mapped.stick_y = coords[1];
    return mapped;
}
