const std = @import("std");
const c = @import("c.zig");
const usb = @import("usb.zig");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Feeder = @import("feeder.zig").Feeder;
const atomic = std.atomic;
const time = std.time;
const print = std.debug.print;

pub const Context = struct {
    feeder: *Feeder,
    stop: atomic.Bool,
};

fn inputLoop(context: *Context) void {
    const feeder = context.feeder;

    while (!context.stop.load(.SeqCst)) {
        _ = feeder.feed() catch {};
    }
}

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
    c.SetConfigFlags(c.FLAG_VSYNC_HINT);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    defer c.CloseWindow();
    c.SetTargetFPS(60);

    var thread_ctx = Context{ .feeder = &feeder, .stop = atomic.Bool.init(false) };

    var thread = try std.Thread.spawn(&thread_ctx, inputLoop);
    defer {
        thread_ctx.stop.store(true, .SeqCst);
        thread.wait();
    }

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        c.ClearBackground(c.DARKGRAY);
    }
}
