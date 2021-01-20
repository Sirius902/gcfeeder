const std = @import("std");
const c = @import("c.zig");
const usb = @import("usb.zig");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Feeder = @import("feeder.zig").Feeder;
const time = std.time;
const print = std.debug.print;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var feeder = try Feeder.init(&ctx);
    defer feeder.deinit();

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    defer c.CloseWindow();
    c.SetTargetFPS(60);

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        c.ClearBackground(c.DARKGRAY);

        const input = feeder.feed();

        {
            const disp = try if (input) |in|
                std.fmt.allocPrintZ(allocator, "trigger_left: {}", .{in.trigger_left})
            else
                std.fmt.allocPrintZ(allocator, "trigger_left: -", .{});

            defer allocator.free(disp);

            c.DrawText(disp, 190, 200, 20, c.LIGHTGRAY);
        }

        {
            const disp = try if (input) |in|
                std.fmt.allocPrintZ(allocator, "trigger_right: {}", .{in.trigger_right})
            else
                std.fmt.allocPrintZ(allocator, "trigger_right: -", .{});

            defer allocator.free(disp);

            c.DrawText(disp, 190, 225, 20, c.LIGHTGRAY);
        }
    }
}
