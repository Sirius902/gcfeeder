const std = @import("std");
const c = @import("c.zig");
const display = @import("display/display.zig");
const usb = @import("zusb");
const vjoy = @import("vjoy.zig");
const Allocator = std.mem.Allocator;
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const atomic = std.atomic;
const time = std.time;
const print = std.debug.print;

pub const Context = struct {
    feeder: *Feeder,
    reciever: *vjoy.FFBReciever,
    stop: atomic.Bool,
    last_input: ?Input = null,
    ess_adapter: bool,
};

fn inputLoop(context: *Context) void {
    const feeder = context.feeder;

    while (!context.stop.load(.Acquire)) {
        context.last_input = feeder.feed(context.ess_adapter) catch |err| blk: {
            print("{} error in input thread\n", .{err});
            break :blk null;
        };
    }
}

fn rumbleLoop(context: *Context) void {
    const feeder = context.feeder;
    const reciever = context.reciever;
    var last_timestamp: ?i64 = null;
    var rumble = Rumble.Off;

    while (!context.stop.load(.Acquire)) {
        if (reciever.get()) |packet| {
            if (packet.device_id == 1) {
                rumble = switch (packet.effect.operation) {
                    .Stop => Rumble.Off,
                    else => Rumble.On,
                };

                if (last_timestamp) |last| {
                    if (packet.timestamp_ms - last < 2) {
                        rumble = Rumble.Off;
                    }
                }

                last_timestamp = packet.timestamp_ms;
            }
        }

        feeder.adapter.setRumble(.{ rumble, .Off, .Off, .Off }) catch |err| {
            print("{} error in rumble thread\n", .{err});
        };
    }
}

pub fn parseArgs(allocator: *Allocator) !bool {
    var ess_adapter = false;

    var iter = std.process.args();
    while (iter.next(allocator)) |arg| {
        const argument = try arg;
        defer allocator.free(argument);

        if (std.mem.eql(u8, argument, "-e")) {
            ess_adapter = true;
        }
    }

    return ess_adapter;
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

    var thread_ctx = Context{
        .feeder = &feeder,
        .reciever = reciever,
        .stop = atomic.Bool.init(false),
        .ess_adapter = try parseArgs(allocator),
    };

    var threads = [_]*std.Thread{
        try std.Thread.spawn(inputLoop, &thread_ctx),
        try std.Thread.spawn(rumbleLoop, &thread_ctx),
    };

    defer {
        thread_ctx.stop.store(true, .Release);

        for (threads) |thread| {
            thread.wait();
        }
    }

    // print("Feeding. Press enter to exit...\n", .{});

    // var reader = std.io.getStdIn().reader();
    // _ = try reader.readByte();

    try display.show(&thread_ctx);
}
