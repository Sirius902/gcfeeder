const std = @import("std");
const zlm = @import("zlm");
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const stick_range = @import("adapter.zig").Calibration.stick_range;

pub const StickCalibration = struct {
    notch_points: [8][2]u8,
    stick_center: [2]u8,

    pub fn map(self: StickCalibration, pos: [2]u8, overscale: ?f32) [2]u8 {
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

        const t = mat3Mul(x, mat3Invert(a) orelse unreachable);
        const res = zlm.Vec3.new(@intToFloat(f32, pos[0]), @intToFloat(f32, pos[1]), 1.0).transform(t);

        if (overscale) |o| {
            const resY = ((res.y - @intToFloat(f32, stick_range.radius)) * o) + @intToFloat(f32, stick_range.radius);
            const resX = ((res.x - @intToFloat(f32, stick_range.radius)) * o) + @intToFloat(f32, stick_range.radius);

            // HACK: Not sure why x and y have to be switched here.
            return [_]u8{ std.math.lossyCast(u8, @round(resY)), std.math.lossyCast(u8, @round(resX)) };
        } else {
            // HACK: Not sure why x and y have to be switched here.
            return [_]u8{ std.math.lossyCast(u8, @round(res.y)), std.math.lossyCast(u8, @round(res.x)) };
        }
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
};

pub const Calibration = struct {
    main_stick: StickCalibration,
    c_stick: StickCalibration,

    pub fn map(self: Calibration, input: Input, overscale: ?f32) Input {
        const main_stick = self.main_stick.map([_]u8{ input.stick_x, input.stick_y }, overscale);
        const c_stick = self.c_stick.map([_]u8{ input.substick_x, input.substick_y }, overscale);

        var res = input;
        res.stick_x = main_stick[0];
        res.stick_y = main_stick[1];
        res.substick_x = c_stick[0];
        res.substick_y = c_stick[1];
        return res;
    }
};

fn waitForA(adapter: *Adapter) !Input {
    var pressed = false;

    while (true) {
        const inputs = try adapter.readInputs();

        if (inputs[0]) |input| {
            if (input.button_a) {
                pressed = true;
            } else if (pressed) {
                return input;
            }
        }
    }
}

pub fn generateCalibration(adapter: *Adapter) !Calibration {
    var calibration: Calibration = undefined;

    std.debug.print("Generating calibration\n", .{});

    var main_stick = true;
    while (true) {
        const stick_name = if (main_stick) "main " else "C-";
        std.debug.print("Center {s}stick and press A\n", .{stick_name});
        {
            const input = try waitForA(adapter);
            if (main_stick) {
                calibration.main_stick.stick_center[0] = input.stick_x;
                calibration.main_stick.stick_center[1] = input.stick_y;
            } else {
                calibration.c_stick.stick_center[0] = input.substick_x;
                calibration.c_stick.stick_center[1] = input.substick_y;
            }
        }

        const notch_names = [_][]const u8{
            "top",
            "top-right",
            "right",
            "bottom-right",
            "bottom",
            "bottom-left",
            "left",
            "top-left",
        };

        for (if (main_stick) calibration.main_stick.notch_points else calibration.c_stick.notch_points) |*p, i| {
            std.debug.print("Move {s}stick to center then to {s} notch then press A\n", .{ stick_name, notch_names[i] });

            const input = try waitForA(adapter);
            p.*[0] = if (main_stick) input.stick_x else input.substick_x;
            p.*[1] = if (main_stick) input.stick_y else input.substick_y;
        }

        if (main_stick) {
            main_stick = false;
        } else {
            break;
        }
    }

    return calibration;
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
