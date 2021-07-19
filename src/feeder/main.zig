const std = @import("std");
const usb = @import("zusb");
const network = @import("network");
const clap = @import("clap");
const vjoy = @import("vjoy.zig");
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const Feeder = @import("feeder.zig").Feeder;
const ess = @import("ess/ess.zig");
const Atomic = std.atomic.Atomic;
const time = std.time;

pub const log_level = .info;

const Options = struct {
    ess_mapping: ?*const ess.NormalizedMap,
    port: ?u16,
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
    mutex: std.Thread.Mutex,
    usb_ctx: *usb.Context,
    feeder: ?Feeder,
    receiver: *vjoy.FFBReceiver,
    stop: Atomic(bool),
    server: ?*Server,
    ess_mapping: ?*const ess.NormalizedMap,
};

const fail_wait = 100 * time.ns_per_ms;

fn inputLoop(context: *Context) void {
    while (!context.stop.load(.Acquire)) {
        if (context.feeder) |*feeder| {
            const input = feeder.feed(context.ess_mapping) catch |err| {
                switch (err) {
                    error.Timeout => continue,
                    else => {
                        const held = context.mutex.acquire();
                        defer held.release();

                        feeder.deinit();
                        context.feeder = null;
                        std.log.err("{} in input thread", .{err});
                        std.log.info("Disconnected from adapter and vJoy", .{});
                        continue;
                    },
                }
            };

            if (context.server) |s| {
                if (input) |in| {
                    var buffer: [@sizeOf(Input)]u8 = undefined;
                    in.serialize(&buffer);

                    _ = s.sock.sendTo(s.endpoint, &buffer) catch |err| {
                        std.log.err("{} in input thread", .{err});
                    };
                }
            }
        } else {
            const held = context.mutex.acquire();
            defer held.release();

            context.feeder = Feeder.init(context.usb_ctx) catch |err| {
                std.log.err("{} in input thread", .{err});
                time.sleep(fail_wait);
                continue;
            };

            std.log.info("Connected to adapter and vJoy", .{});
        }
    }
}

fn rumbleLoop(context: *Context) void {
    const receiver = context.receiver;
    var last_timestamp: ?i64 = null;
    var rumble = Rumble.Off;

    while (!context.stop.load(.Acquire)) {
        if (context.feeder) |*feeder| {
            if (receiver.get()) |packet| {
                if (packet.device_id == 1) {
                    rumble = switch (packet.effect.operation) {
                        .Stop => .Off,
                        else => .On,
                    };

                    if (last_timestamp) |last| {
                        if (packet.timestamp_ms - last < 2) {
                            rumble = .Off;
                        }
                    }

                    last_timestamp = packet.timestamp_ms;
                }
            }

            const held = context.mutex.acquire();

            feeder.adapter.setRumble(.{ rumble, .Off, .Off, .Off }) catch |err| {
                switch (err) {
                    error.Timeout => {
                        held.release();
                        continue;
                    },
                    else => {
                        // Release mutex before sleeping to allow input thread to acquire.
                        held.release();
                        std.log.err("{} in rumble thread", .{err});
                        time.sleep(fail_wait);
                        continue;
                    },
                }
            };

            held.release();
        } else {
            time.sleep(8 * time.ns_per_ms);
        }
    }
}

pub fn main() !void {
    const options = blk: {
        const params = comptime [_]clap.Param(clap.Help){
            clap.parseParam("-h, --help          Display this help and exit.") catch unreachable,
            clap.parseParam("-e, --ess           Enables ESS adapter with oot-vc mapping.") catch unreachable,
            clap.parseParam("-m, --mapping <MAP> Enables ESS adapter with the specified mapping. Available mappings are: oot-vc, mm-vc, z64-gc.") catch unreachable,
            clap.parseParam("-s, --server        Enables UDP input server.") catch unreachable,
            clap.parseParam("-p, --port <PORT>   Enables UDP input server on port.") catch unreachable,
        };

        var args = clap.parse(clap.Help, &params, .{}) catch {
            std.log.err("Invalid arguments. Run with --help to see proper usage.", .{});
            return;
        };
        defer args.deinit();

        if (args.flag("--help")) {
            try clap.help(std.io.getStdErr().writer(), &params);
            return;
        }

        const port = if (args.option("--port")) |p|
            std.fmt.parseUnsigned(u16, p, 10) catch {
                std.log.err("Invalid port specified.", .{});
                return;
            }
        else if (args.flag("--server"))
            @as(u16, 4096)
        else
            null;

        const ess_mapping = if (args.option("--mapping")) |m|
            &(ess.mappings.get(m) orelse {
                std.log.err("Invalid mapping specified.", .{});
                return;
            })
        else if (args.flag("--ess"))
            &ess.mappings.get("oot-vc").?
        else
            null;

        break :blk Options{
            .ess_mapping = ess_mapping,
            .port = port,
        };
    };

    std.log.info("Initializing. Press enter to exit...", .{});

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var receiver = try vjoy.FFBReceiver.init(allocator);
    defer receiver.deinit();

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
        .mutex = std.Thread.Mutex{},
        .usb_ctx = &ctx,
        .feeder = null,
        .receiver = receiver,
        .stop = Atomic(bool).init(false),
        .server = if (server) |*s| s else null,
        .ess_mapping = options.ess_mapping,
    };
    defer if (thread_ctx.feeder) |feeder| feeder.deinit();

    var threads = [_]std.Thread{
        try std.Thread.spawn(.{}, inputLoop, .{&thread_ctx}),
        try std.Thread.spawn(.{}, rumbleLoop, .{&thread_ctx}),
    };

    defer {
        thread_ctx.stop.store(true, .Release);

        for (threads) |thread| {
            thread.join();
        }
    }

    // Wait for user to press enter to exit program.
    _ = try std.io.getStdIn().reader().readByte();
}
