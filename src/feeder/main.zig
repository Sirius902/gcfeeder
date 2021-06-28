const std = @import("std");
const usb = @import("zusb");
const network = @import("network");
const clap = @import("clap");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const Atomic = std.atomic.Atomic;
const time = std.time;

const Options = struct {
    ess_adapter: bool = false,
    port: ?u16 = null,
};

const Server = struct {
    sock: network.Socket,
    endpoint: network.EndPoint,

    pub fn init(port: u16) !Server {
        var sock = try network.Socket.create(.ipv4, .udp);
        const endpoint = network.EndPoint{
            .address = .{ .ipv4 = network.Address.IPv4.loopback },
            .port = port,
        };

        return Server{ .sock = sock, .endpoint = endpoint };
    }

    pub fn deinit(self: Server) void {
        self.sock.close();
    }
};

pub const Context = struct {
    feeder: *Feeder,
    reciever: *vjoy.FFBReciever,
    stop: Atomic(bool),
    server: ?*Server,
    ess_adapter: bool,
};

fn inputLoop(context: *Context) void {
    const feeder = context.feeder;

    while (!context.stop.load(.Acquire)) {
        const input = feeder.feed(context.ess_adapter) catch |err| blk: {
            std.log.err("{} error in input thread", .{err});
            break :blk null;
        };

        if (input) |in| {
            if (context.server) |s| {
                var buffer: [@sizeOf(Input)]u8 = undefined;
                in.serialize(&buffer);

                _ = s.sock.sendTo(s.endpoint, &buffer) catch |err| {
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

    const options = blk: {
        const params = comptime [_]clap.Param(clap.Help){
            clap.parseParam("-h, --help        Display this help and exit.      ") catch unreachable,
            clap.parseParam("-e, --ess         Enables ESS adapter.             ") catch unreachable,
            clap.parseParam("-s, --server      Enables UDP input server.        ") catch unreachable,
            clap.parseParam("-p, --port <PORT> Enables UDP input server on port.") catch unreachable,
        };

        var args = try clap.parse(clap.Help, &params, .{});
        defer args.deinit();

        if (args.flag("--help")) {
            try clap.help(std.io.getStdErr().writer(), &params);
            return;
        }

        const port = if (args.option("--port")) |p|
            try std.fmt.parseUnsigned(u16, p, 10)
        else if (args.flag("--server"))
            @as(u16, 4096)
        else
            null;

        break :blk Options{
            .ess_adapter = args.flag("--ess"),
            .port = port,
        };
    };

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var feeder = try Feeder.init(&ctx);
    defer feeder.deinit();

    var reciever = try vjoy.FFBReciever.init(allocator);
    defer reciever.deinit();

    var server: ?Server = null;
    if (options.port) |p| {
        try network.init();

        server = try Server.init(p);
        std.log.info("Opened UDP server on port {}", .{p});
    }
    defer {
        if (server) |s| {
            s.deinit();
            network.deinit();
        }
    }

    var thread_ctx = Context{
        .feeder = &feeder,
        .reciever = reciever,
        .stop = Atomic(bool).init(false),
        .server = if (server) |*s| s else null,
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
    _ = try std.io.getStdIn().reader().readByte();
}
