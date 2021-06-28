const std = @import("std");
const usb = @import("zusb");
const network = @import("network");
const clap = @import("clap");
const vjoy = @import("vjoy.zig");
const Allocator = std.mem.Allocator;
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const Atomic = std.atomic.Atomic;
const time = std.time;

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

    const options = blk: {
        const params = comptime [_]clap.Param(clap.Help){
            clap.parseParam("-h, --help   Display this help and exit.") catch unreachable,
            clap.parseParam("-e, --ess    Enables ESS adapter.       ") catch unreachable,
            clap.parseParam("-s, --server Enables UDP input server.  ") catch unreachable,
        };

        var args = try clap.parse(clap.Help, &params, .{});
        defer args.deinit();

        if (args.flag("--help")) {
            try clap.help(std.io.getStdErr().writer(), &params);
            return;
        } else {
            break :blk Options{
                .ess_adapter = args.flag("--ess"),
                .input_server = args.flag("--server"),
            };
        }
    };

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
        .sock = if (sock) |*s| s else null,
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
