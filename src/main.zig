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
    stop: *atomic.Bool,
};

fn feederLoop(context: Context) void {
    const feeder = context.feeder;
    var was_error = false;

    while (!context.stop.load(.SeqCst)) {
        if (was_error) {
            was_error = false;
            var ctx = usb.Context{ .ctx = feeder.adapter.handle.ctx };

            if (Adapter.init(&ctx)) |adapter| {
                feeder.adapter.deinit();
                feeder.adapter = adapter;
            } else |_| {
                was_error = true;
            }
        } else {
            _ = feeder.feed() catch |err| {
                switch (err) {
                    usb.Error.Io => was_error = true,
                    else => {},
                }
            };
        }
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

    var stop = atomic.Bool.init(false);

    var thread = try std.Thread.spawn(Context{ .feeder = &feeder, .stop = &stop }, feederLoop);
    defer {
        stop.store(true, .SeqCst);
        thread.wait();
    }

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        c.ClearBackground(c.DARKGRAY);
    }
}
