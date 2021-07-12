const std = @import("std");
const Input = @import("../adapter.zig").Input;
const main_stick = @import("../adapter.zig").Calibration.main_stick;
const vc_map: []const u8 = @embedFile("map/oot-vc.bin");

/// Maps first quadrant normalized GC coordinates to fit to the shape of the N64 controller.
/// Uses math from https://github.com/Skuzee/ESS-Adapter.
///
/// Precondition: `0 <= y <= x <= 127`
fn gcToN64(coords: *[2]u8) void {
    const x = @intToFloat(f64, coords[0]);
    const y = @intToFloat(f64, coords[1]);

    const scale = std.math.pow(f64, (5.0 * x + 2.0 * y) / 525.0, 2.0) * (7.0 * y / 525.0) * (70.0 / 75.0 - 80.0 / 105.0) + 80.0 / 105.0;

    coords[0] = @floatToInt(u8, @ceil(x * scale));
    coords[1] = @floatToInt(u8, @ceil(y * scale));
}

pub fn map(input: Input) Input {
    const x_positive = input.stick_x >= 128;
    const y_positive = input.stick_y >= 128;
    const swap = input.stick_y > input.stick_x;

    var coords = [_]u8{ input.stick_x, input.stick_y };

    if (x_positive) {
        coords[0] -= 128;
    } else {
        if (coords[0] == 0) {
            coords[0] = 127;
        } else {
            coords[0] = 128 - coords[0];
        }
    }

    if (y_positive) {
        coords[1] -= 128;
    } else {
        if (coords[1] == 0) {
            coords[1] = 127;
        } else {
            coords[1] = 128 - coords[1];
        }
    }

    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    gcToN64(&coords);

    if (swap) std.mem.swap(u8, &coords[0], &coords[1]);

    if (x_positive) {
        coords[0] += 128;
    } else {
        coords[0] = 128 - coords[0];
    }

    if (y_positive) {
        coords[1] += 128;
    } else {
        coords[1] = 128 - coords[1];
    }

    const map_index = 2 * ((@as(usize, coords[1]) * 256) + coords[0]);
    coords[0] = vc_map[map_index];
    coords[1] = vc_map[map_index + 1];

    var mapped = input;
    mapped.stick_x = coords[0];
    mapped.stick_y = coords[1];
    return mapped;
}
