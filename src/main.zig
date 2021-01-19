const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");
const usb = @import("usb.zig");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var adapter = try Adapter.init(&ctx);
    defer adapter.deinit();

    var vjd = try vjoy.Device.init(1);
    defer vjd.deinit();

    var pos: vjoy.JoystickPosition = undefined;
    @memset(@ptrCast([*]u8, &pos), 0, @sizeOf(@TypeOf(pos)));

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    c.SetTargetFPS(60);

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        c.ClearBackground(c.DARKGRAY);

        const inputs = adapter.readInputs() catch null;
        const input = if (inputs) |in| in[0] else null;

        if (input) |in| {
            pos.wAxisX = @as(c_long, in.stick_x) * 0x7F;
            pos.wAxisY = @as(c_long, ~in.stick_y) * 0x7F;

            _ = vjd.update(pos) catch {};
        }

        {
            const disp = try if (input) |in|
                std.fmt.allocPrintZ(allocator, "stick_x: {}", .{in.stick_x})
            else
                std.fmt.allocPrintZ(allocator, "stick_x: -", .{});

            defer allocator.free(disp);

            c.DrawText(disp, 190, 200, 20, c.LIGHTGRAY);
        }

        {
            const disp = try if (input) |in|
                std.fmt.allocPrintZ(allocator, "stick_y: {}", .{in.stick_y})
            else
                std.fmt.allocPrintZ(allocator, "stick_y: -", .{});

            defer allocator.free(disp);

            c.DrawText(disp, 190, 225, 20, c.LIGHTGRAY);
        }
    }

    c.CloseWindow();
}
