const std = @import("std");
const usb = @import("zusb");
const network = @import("network");
const vjoy = @import("vjoy.zig");
const Allocator = std.mem.Allocator;
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const Atomic = std.atomic.Atomic;
const time = std.time;

pub const Error = error{
    InvalidArgument,
};

pub const Options = struct {
    ess_adapter: bool = false,
    input_server: bool = false,
};

pub const Context = struct {
    feeder: *Feeder,
    reciever: *vjoy.FFBReciever,
    stop: Atomic(bool),
    sock: ?*network.Socket,
    ess_adapter: bool,
};

const endpoint = network.EndPoint{
    .address = .{ .ipv4 = network.Address.IPv4.loopback },
    .port = 4096,
};

fn inputLoop(context: *Context) void {
    const feeder = context.feeder;

    while (!context.stop.load(.Acquire)) {
        const input = feeder.feed(context.ess_adapter) catch |err| blk: {
            std.log.err("{} error in input thread", .{err});
            break :blk null;
        };

        if (input) |in| {
            if (context.sock) |s| {
                var buffer: [@sizeOf(Input)]u8 = undefined;
                in.serialize(&buffer);

                _ = s.sendTo(endpoint, &buffer) catch |err| {
                    std.log.err("{} error in input thread", .{err});
                };
            }
        }
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
            std.log.err("{} error in rumble thread", .{err});
        };
    }
}

pub fn parseArgs(allocator: *Allocator) !Options {
    var options = Options{};

    var iter = std.process.args();
    while (iter.next(allocator)) |arg| {
        const argument = try arg;
        defer allocator.free(argument);

        if (std.mem.startsWith(u8, argument, "-")) {
            for (argument[1..]) |opt| {
                switch (opt) {
                    'e' => {
                        options.ess_adapter = true;
                    },
                    'i' => {
                        options.input_server = true;
                    },
                    else => {
                        return Error.InvalidArgument;
                    },
                }
            }
        }
    }

    return options;
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

    const options = try parseArgs(allocator);

    var sock: ?network.Socket = null;
    if (options.input_server) {
        try network.init();

        sock = try network.Socket.create(.ipv4, .udp);

        try sock.?.connect(endpoint);
        std.log.info("Opened UDP server on port {}", .{endpoint.port});
    }
    defer {
        if (options.input_server) network.deinit();
        if (sock) |s| s.close();
    }

    var thread_ctx = Context{
        .feeder = &feeder,
        .reciever = reciever,
        .stop = Atomic(bool).init(false),
        .sock = if (sock) |_| &sock.? else null,
        .ess_adapter = options.ess_adapter,
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

    std.log.info("Feeding. Press enter to exit...", .{});

    var reader = std.io.getStdIn().reader();
    _ = try reader.readByte();
}
