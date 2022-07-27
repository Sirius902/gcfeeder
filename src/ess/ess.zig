const std = @import("std");
const math = std.math;
const Tuple = std.meta.Tuple;
const Input = @import("../adapter.zig").Input;
const stick_range = @import("../adapter.zig").Calibration.stick_range;

pub const Mapping = enum {
    oot_vc,
    mm_vc,
    z64_gc,

    pub fn fileName(comptime self: Mapping) []const u8 {
        comptime {
            const tag_name = std.meta.tagName(self);
            var name: [tag_name.len]u8 = undefined;

            for (tag_name) |c, i| {
                name[i] = switch (c) {
                    '_' => '-',
                    else => c,
                };
            }

            return &name;
        }
    }

    pub fn fromFileName(file_name: []const u8) ?Mapping {
        inline for (comptime std.enums.values(Mapping)) |variant| {
            if (std.mem.eql(u8, file_name, variant.fileName())) {
                return variant;
            }
        }

        return null;
    }

    pub fn normalizedMap(self: Mapping) NormalizedMap {
        inline for (comptime std.enums.values(Mapping)) |variant| {
            if (self == variant) {
                return NormalizedMap{ .table = @embedFile("map/" ++ variant.fileName() ++ ".bin") };
            }
        }

        unreachable;
    }

    pub fn jsonStringify(
        value: Mapping,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        _ = options;
        try out_stream.writeByte('"');

        comptime var written = false;
        inline for (comptime std.enums.values(Mapping)) |variant| {
            if (value == variant) {
                try out_stream.writeAll(comptime variant.fileName());
                written = true;
            }
        }

        if (!written) {
            unreachable;
        }

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
    One,
    Two,
    Three,
    Four,

    pub fn of(coords: *const [2]u8) Quadrant {
        if (coords[0] >= stick_range.center) {
            return if (coords[1] >= stick_range.center) .One else .Four;
        } else {
            return if (coords[1] >= stick_range.center) .Two else .Three;
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
        .One, .Four => x - stick_range.center,
        .Two, .Three => math.min(stick_range.center - x, stick_range.radius),
    };

    const yy = switch (original) {
        .One, .Two => y - stick_range.center,
        .Three, .Four => math.min(stick_range.center - y, stick_range.radius),
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
        .One, .Four => x + stick_range.center,
        .Two, .Three => stick_range.center - x,
    };

    const yy = switch (original) {
        .One, .Two => y + stick_range.center,
        .Three, .Four => stick_range.center - y,
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

pub fn map(mapping: Mapping, input: Input) Input {
    const swap = input.stick_y > input.stick_x;
    var coords = [_]u8{ input.stick_x, input.stick_y };

    const q = normalize(&coords);
    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    gcToN64(&coords);

    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    mapping.normalizedMap().map(&coords);

    denormalize(&coords, q);

    var mapped = input;
    mapped.stick_x = coords[0];
    mapped.stick_y = coords[1];
    return mapped;
}
