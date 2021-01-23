const std = @import("std");
const c = @import("c.zig");
const usb = @import("usb.zig");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const atomic = std.atomic;
const time = std.time;
const print = std.debug.print;

pub const Context = struct {
    feeder: *Feeder,
    reciever: *vjoy.FFBReciever,
    stop: atomic.Bool,
};

fn inputLoop(context: *Context) void {
    const feeder = context.feeder;

    while (!context.stop.load(.SeqCst)) {
        _ = feeder.feed() catch {};
    }
}

fn rumbleLoop(context: *Context) void {
    const feeder = context.feeder;
    const reciever = context.reciever;
    var last_timestamp: ?i64 = null;
    var rumble = Rumble.Off;

    while (!context.stop.load(.SeqCst)) {
        if (reciever.get()) |packet| {
            switch (packet) {
                .effect_operation => |eff_op| {
                    if (eff_op.device_id == 1) {
                        rumble = switch (eff_op.operation.EffectOp) {
                            .EFF_STOP => Rumble.Off,
                            else => Rumble.On,
                        };

                        if (last_timestamp) |last| {
                            if (eff_op.timestamp_ms - last < 2) {
                                rumble = Rumble.Off;
                            }
                        }

                        last_timestamp = eff_op.timestamp_ms;
                    }
                },
                else => {},
            }
        }

        feeder.adapter.setRumble(.{ rumble, .Off, .Off, .Off }) catch {};
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

    var reciever = try vjoy.FFBReciever.init(allocator);
    defer reciever.deinit();

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.SetConfigFlags(c.FLAG_VSYNC_HINT);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    defer c.CloseWindow();
    c.SetTargetFPS(60);

    var thread_ctx = Context{
        .feeder = &feeder,
        .reciever = reciever,
        .stop = atomic.Bool.init(false),
    };

    var threads = [_]*std.Thread{
        try std.Thread.spawn(&thread_ctx, inputLoop),
        try std.Thread.spawn(&thread_ctx, rumbleLoop),
    };

    defer {
        thread_ctx.stop.store(true, .SeqCst);

        for (threads) |thread| {
            thread.wait();
        }
    }

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        c.ClearBackground(c.DARKGRAY);
    }
}
