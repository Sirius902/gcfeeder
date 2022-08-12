const std = @import("std");
const zlm = @import("zlm");
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const stick_range = @import("adapter.zig").Calibration.stick_range;

pub const StickCalibration = struct {
    notch_points: [8][2]u8,
    stick_center: [2]u8,

    pub const Error = error{
        BadCalibration,
    };

    pub fn map(self: StickCalibration, pos: [2]u8) Error![2]u8 {
        const q = self.quadrant(pos);
        const qn = (q + self.notch_points.len - 1) % self.notch_points.len;

        const stick_center = zlm.Vec2.new(@intToFloat(f32, self.stick_center[0]), @intToFloat(f32, self.stick_center[1]));

        const left_point = zlm.Vec2.new(@intToFloat(f32, self.notch_points[q][0]), @intToFloat(f32, self.notch_points[q][1]));
        const right_point = zlm.Vec2.new(@intToFloat(f32, self.notch_points[qn][0]), @intToFloat(f32, self.notch_points[qn][1]));

        const theta1 = @intToFloat(f32, q) * std.math.pi / 4.0;
        const theta2 = @intToFloat(f32, qn) * std.math.pi / 4.0;

        const d = [_]zlm.Vec2{
            zlm.Vec2.new(@intToFloat(f32, stick_range.center), @intToFloat(f32, stick_range.center)),
            zlm.Vec2.new(
                @intToFloat(f32, stick_range.radius) * @cos(theta1) + @intToFloat(f32, stick_range.center),
                @intToFloat(f32, stick_range.radius) * @sin(theta1) + @intToFloat(f32, stick_range.center),
            ),
            zlm.Vec2.new(
                @intToFloat(f32, stick_range.radius) * @cos(theta2) + @intToFloat(f32, stick_range.center),
                @intToFloat(f32, stick_range.radius) * @sin(theta2) + @intToFloat(f32, stick_range.center),
            ),
        };

        const sx1 = stick_center.x;
        const sy1 = stick_center.y;
        const sx2 = left_point.x;
        const sy2 = left_point.y;
        const sx3 = right_point.x;
        const sy3 = right_point.y;
        const dx1 = d[0].x;
        const dy1 = d[0].y;
        const dx2 = d[1].x;
        const dy2 = d[1].y;
        const dx3 = d[2].x;
        const dy3 = d[2].y;

        const a = zlm.Mat3{ .fields = .{
            .{ sx1, sx2, sx3 },
            .{ sy1, sy2, sy3 },
            .{ 1.0, 1.0, 1.0 },
        } };

        const x = zlm.Mat3{ .fields = .{
            .{ dx1, dx2, dx3 },
            .{ dy1, dy2, dy3 },
            .{ 1.0, 1.0, 1.0 },
        } };

        const t = mat3Mul(x, mat3Invert(a) orelse return error.BadCalibration);
        const res = zlm.Vec3.new(@intToFloat(f32, pos[0]), @intToFloat(f32, pos[1]), 1.0).transform(t);

        // HACK: Not sure why x and y have to be switched here.
        return [_]u8{ std.math.lossyCast(u8, @round(res.y)), std.math.lossyCast(u8, @round(res.x)) };
    }

    fn quadrant(self: StickCalibration, pos: [2]u8) u3 {
        var angles: [8]f32 = undefined;
        for (angles) |*a, i| {
            const dx = @intToFloat(f32, self.notch_points[i][0]) - @intToFloat(f32, self.stick_center[0]);
            const dy = @intToFloat(f32, self.notch_points[i][1]) - @intToFloat(f32, self.stick_center[1]);
            a.* = std.math.atan2(f32, dy, dx);
        }

        const dx = @intToFloat(f32, pos[0]) - @intToFloat(f32, self.stick_center[0]);
        const dy = @intToFloat(f32, pos[1]) - @intToFloat(f32, self.stick_center[1]);
        const angle = std.math.atan2(f32, dy, dx);

        const max_index = blk: {
            var i: usize = 0;
            for (angles[1..]) |a, j| {
                if (angles[i] < a) {
                    i = j;
                }
            }
            break :blk i;
        };

        const max_angle = angles[max_index];
        const min_angle = angles[(max_index + angles.len - 1) % angles.len];

        if (angle > max_angle or angle < min_angle) {
            return @intCast(u3, max_index);
        }

        var i: usize = 0;
        while (i < angles.len) : (i += 1) {
            if (i == max_index) continue;

            const start_angle = angles[i];
            const end_angle = angles[(i + angles.len - 1) % angles.len];

            if (angle >= start_angle and angle <= end_angle) {
                return @intCast(u3, i);
            }
        }

        unreachable;
    }

    pub fn jsonStringify(
        value: StickCalibration,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        const T = @TypeOf(value);
        const S = @typeInfo(T).Struct;

        try out_stream.writeByte('{');
        var field_output = false;
        var child_options = options;
        if (child_options.whitespace) |*child_whitespace| {
            child_whitespace.indent_level += 1;
        }
        inline for (S.fields) |Field| {
            // don't include void fields
            if (Field.field_type == void) continue;

            var emit_field = true;

            // don't include optional fields that are null when emit_null_optional_fields is set to false
            if (@typeInfo(Field.field_type) == .Optional) {
                if (options.emit_null_optional_fields == false) {
                    if (@field(value, Field.name) == null) {
                        emit_field = false;
                    }
                }
            }

            if (emit_field) {
                if (!field_output) {
                    field_output = true;
                } else {
                    try out_stream.writeByte(',');
                }
                if (child_options.whitespace) |child_whitespace| {
                    try child_whitespace.outputIndent(out_stream);
                }
                try std.json.stringify(Field.name, options, out_stream);
                try out_stream.writeByte(':');
                if (child_options.whitespace) |child_whitespace| {
                    if (child_whitespace.separator) {
                        try out_stream.writeByte(' ');
                    }
                }
                try std.json.stringify(@field(value, Field.name), .{ .string = .Array }, out_stream);
            }
        }
        if (field_output) {
            if (options.whitespace) |whitespace| {
                try whitespace.outputIndent(out_stream);
            }
        }
        try out_stream.writeByte('}');
    }
};

pub const Calibration = struct {
    main_stick: StickCalibration,
    c_stick: StickCalibration,

    pub const Error = StickCalibration.Error;

    pub fn map(self: Calibration, input: Input) Error!Input {
        const main_stick = try self.main_stick.map([_]u8{ input.stick_x, input.stick_y });
        const c_stick = try self.c_stick.map([_]u8{ input.substick_x, input.substick_y });

        var res = input;
        res.stick_x = main_stick[0];
        res.stick_y = main_stick[1];
        res.substick_x = c_stick[0];
        res.substick_y = c_stick[1];
        return res;
    }
};

pub fn applyScaling(input: Input, analog_scale: f32) Input {
    const main_scaled = applyStickScaling([_]u8{ input.stick_x, input.stick_y }, analog_scale);
    const c_scaled = applyStickScaling([_]u8{ input.substick_x, input.substick_y }, analog_scale);

    var result = input;
    result.stick_x = main_scaled[0];
    result.stick_y = main_scaled[1];
    result.substick_x = c_scaled[0];
    result.substick_y = c_scaled[1];
    return result;
}

fn applyStickScaling(coord: [2]u8, analog_scale: f32) [2]u8 {
    const radius = @intToFloat(f32, stick_range.radius);
    const x = ((@intToFloat(f32, coord[0]) - radius) * analog_scale) + radius;
    const y = ((@intToFloat(f32, coord[1]) - radius) * analog_scale) + radius;

    return [_]u8{ std.math.lossyCast(u8, @round(x)), std.math.lossyCast(u8, @round(y)) };
}

fn mat3Invert(m: zlm.Mat3) ?zlm.Mat3 {
    var m4 = zlm.Mat4{ .fields = .{
        .{ 0.0, 0.0, 0.0, 0.0 },
        .{ 0.0, 0.0, 0.0, 0.0 },
        .{ 0.0, 0.0, 0.0, 0.0 },
        .{ 0.0, 0.0, 0.0, 1.0 },
    } };

    inline for ([_]comptime_int{ 0, 1, 2 }) |row| {
        std.mem.copy(f32, &m4.fields[row], &m.fields[row]);
    }

    const inv = m4.invert() orelse return null;

    var ret: zlm.Mat3 = undefined;
    inline for ([_]comptime_int{ 0, 1, 2 }) |row| {
        inline for ([_]comptime_int{ 0, 1, 2 }) |column| {
            ret.fields[row][column] = inv.fields[row][column];
        }
    }

    return ret;
}

fn mat3Mul(a: zlm.Mat3, b: zlm.Mat3) zlm.Mat3 {
    var result: zlm.Mat3 = undefined;
    inline for ([_]comptime_int{ 0, 1, 2 }) |row| {
        inline for ([_]comptime_int{ 0, 1, 2 }) |col| {
            var sum: f32 = 0.0;
            inline for ([_]comptime_int{ 0, 1, 2 }) |i| {
                sum += a.fields[row][i] * b.fields[i][col];
            }
            result.fields[row][col] = sum;
        }
    }
    return result;
}
