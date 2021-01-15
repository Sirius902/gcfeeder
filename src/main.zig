const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");
const usb = @import("usb.zig");
const Adapter = @import("adapter.zig").Adapter;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var adapter = try Adapter.init(&ctx);
    defer adapter.deinit();

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    c.SetTargetFPS(60);

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        const input = (try adapter.readInputs())[0];

        const disp = if (input) |in|
            try std.fmt.allocPrintZ(allocator, "stick_x: {}", .{in.stick_x})
        else
            "stick_x: -";

        defer allocator.free(disp);

        c.ClearBackground(c.DARKGRAY);
        c.DrawText(disp, 190, 200, 20, c.LIGHTGRAY);
    }

    c.CloseWindow();
}
