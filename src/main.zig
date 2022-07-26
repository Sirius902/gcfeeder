const std = @import("std");
const builtin = @import("builtin");
const build_info = @import("build_info");
const usb = @import("zusb");
const clap = @import("clap");
const calibrate = @import("calibrate.zig");
const Calibration = @import("calibrate.zig").Calibration;
const Bridge = @import("bridge/bridge.zig").Bridge;
const ViGEmBridge = @import("bridge/bridge.zig").ViGEmBridge;
const Adapter = @import("adapter.zig").Adapter;
const Input = @import("adapter.zig").Input;
const Rumble = @import("adapter.zig").Rumble;
const ess = @import("ess/ess.zig");
const Atomic = std.atomic.Atomic;
const time = std.time;
const emulator = @import("emulator.zig");
const ConfigFile = @import("config.zig").ConfigFile;
const Config = @import("config.zig").Config;
const gui = @import("gui/gui.zig");
const win = @cImport({
    @cDefine("WIN32_LEAN_AND_MEAN", {});
    @cInclude("windows.h");
});

pub const log = gui.log;
pub const log_level = if (builtin.mode == .Debug) .debug else .info;

const Options = struct {
    ess_mapping: ?ess.Mapping,
    port: ?u16,
    use_calibration: bool,
    emulator_rumble: bool,
    overscale: ?f32,
};

pub const Context = struct {
    allocator: std.mem.Allocator,
    config_arena: ?std.heap.ArenaAllocator = null,
    mutex: std.Thread.Mutex,
    usb_ctx: *usb.Context,
    adapter: ?Adapter = null,
    bridge: ?Bridge = null,
    config_reloading: Atomic(bool) = Atomic(bool).init(false),
    stop: Atomic(bool) = Atomic(bool).init(false),
    sock: ?*const std.x.os.Socket,
    ess_mapping: ?ess.Mapping,
    config_file: ?ConfigFile = null,
    config: ?*Config = null,
    use_calibration: bool,
    emulator_rumble: bool,
    overscale: ?f32,
};

const fail_wait = 100 * time.ns_per_ms;

fn loadAndSetConfig(context: *Context) !void {
    context.config_reloading.store(true, .Release);
    defer context.config_reloading.store(false, .Release);
    context.mutex.lock();
    defer context.mutex.unlock();

    var config_arena = std.heap.ArenaAllocator.init(context.allocator);
    const json_allocator = config_arena.allocator();

    const config_file = (try ConfigFile.load(json_allocator)) orelse blk: {
        const c = try ConfigFile.init(json_allocator, .{});
        c.save(json_allocator) catch |err| {
            // TODO: error to gui
            std.debug.panic("Failed to save {s}: {}", .{ ConfigFile.path, err });
        };
        break :blk c;
    };

    const profile_name = config_file.current_profile;
    const profile = config_file.lookupProfile(profile_name) orelse {
        // TODO: error to gui
        std.debug.panic("Missing profile \"{s}\"", .{profile_name});
    };

    if (context.bridge) |bridge| {
        bridge.deinit();
        context.bridge = null;
    }

    if (context.config_arena) |prev_arena| {
        prev_arena.deinit();
    }

    context.config_arena = config_arena;
    context.config_file = config_file;
    context.config = &profile.config;

    std.log.info("Config loaded with profile \"{s}\"", .{profile_name});
    gui.notifyReload();
}

fn inputLoop(context: *Context) void {
    gui.waitForInit();

    while (!context.stop.load(.Acquire)) {
        if (context.config == null or gui.isReloadNeeded()) {
            loadAndSetConfig(context) catch |err| {
                std.debug.panic("Failed to load config: {}", .{err});
            };
        }

        const adapter = &(context.adapter orelse blk: {
            context.mutex.lock();
            defer context.mutex.unlock();

            const a = Adapter.init(context.usb_ctx) catch |err| {
                std.log.err("{} in input thread", .{err});
                time.sleep(fail_wait);
                continue;
            };

            std.log.info("Connected to adapter", .{});

            context.adapter = a;
            break :blk a;
        });

        const config_file = &context.config_file.?;
        const config = context.config.?;

        const bridge = context.bridge orelse blk: {
            context.mutex.lock();
            defer context.mutex.unlock();

            const b = switch (config.driver) {
                .vigem => ViGEmBridge.initBridge(context.allocator, config.vigem_config),
            } catch |err| {
                std.log.err("{} in input thread", .{err});
                time.sleep(fail_wait);
                continue;
            };

            std.log.info("Connected to {s}", .{b.driverName()});

            context.bridge = b;
            break :blk b;
        };

        // TODO: Calibrate when requested from gui
        if (context.use_calibration and config.calibration == null) {
            const cal = calibrate.generateCalibration(adapter) catch |err| {
                switch (err) {
                    error.Timeout => continue,
                    else => {
                        context.mutex.lock();
                        defer context.mutex.unlock();

                        adapter.deinit();
                        context.adapter = null;
                        std.log.err("{} in input thread", .{err});
                        std.log.info("Disconnected from adapter", .{});
                        continue;
                    },
                }
            };
            config.calibration = cal;
            config_file.save(context.allocator) catch |err| {
                // TODO: error to gui
                std.debug.panic("Failed to save \"{s}\": {}", .{ ConfigFile.path, err });
            };
        }

        const inputs = adapter.readInputs() catch |err| {
            switch (err) {
                error.Timeout => continue,
                else => {
                    context.mutex.lock();
                    defer context.mutex.unlock();

                    adapter.deinit();
                    context.adapter = null;
                    std.log.err("{} in input thread", .{err});
                    std.log.info("Disconnected from adapter", .{});
                    continue;
                },
            }
        };

        if (inputs[0]) |input| {
            const ess_mapped = if (context.ess_mapping) |m| ess.map(m, input) else input;
            const calibrated = if (context.use_calibration)
                config.calibration.?.map(ess_mapped, context.overscale)
            else
                ess_mapped;

            bridge.feed(calibrated) catch |err| {
                context.mutex.lock();
                defer context.mutex.unlock();

                bridge.deinit();
                context.bridge = null;
                std.log.err("{} in input thread", .{err});
                std.log.info("Disconnected from {s}", .{bridge.driverName()});
                continue;
            };

            if (context.sock) |s| {
                var buffer: [@sizeOf(Input)]u8 = undefined;
                input.serialize(&buffer);

                _ = s.write(&buffer, 0) catch |err| {
                    std.log.err("{} in input thread", .{err});
                };
            }
        }
    }
}

fn rumbleLoop(context: *Context) void {
    var rumble = Rumble.Off;

    var handle: ?emulator.Handle = null;
    defer if (handle) |h| h.close();

    while (!context.stop.load(.Acquire)) {
        if (context.config_reloading.load(.Acquire) or context.adapter == null or context.bridge == null) {
            time.sleep(8 * time.ns_per_ms);
            continue;
        }

        const adapter = &context.adapter.?;
        const bridge = context.bridge.?;

        if (!context.emulator_rumble) {
            if (bridge.pollRumble()) |r| {
                rumble = r;
            }
        } else {
            if (handle) |h| {
                rumble = h.rumbleState() catch blk: {
                    std.log.info("Disconnected from {s}", .{h.emulatorTitle()});
                    h.close();
                    handle = null;
                    break :blk .Off;
                };
            } else {
                handle = emulator.Handle.open() catch blk: {
                    time.sleep(fail_wait);
                    break :blk null;
                };
                if (handle) |h| std.log.info("Connected to {s} OoT 1.0", .{h.emulatorTitle()});
                rumble = .Off;
            }
        }

        context.mutex.lock();

        adapter.setRumble(.{ rumble, .Off, .Off, .Off }) catch |err| {
            switch (err) {
                error.Timeout => {
                    context.mutex.unlock();
                    continue;
                },
                else => {
                    // Release mutex before sleeping to allow input thread to acquire.
                    context.mutex.unlock();
                    std.log.err("{} in rumble thread", .{err});
                    time.sleep(fail_wait);
                    continue;
                },
            }
        };

        context.mutex.unlock();
    }

    context.mutex.lock();
    defer context.mutex.unlock();

    if (context.adapter) |adapter| {
        adapter.setRumble(.{ .Off, .Off, .Off, .Off }) catch {};
    }
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    gui.log_allocator = allocator;

    const options = blk: {
        const params = comptime clap.parseParamsComptime(
            \\-h, --help            Display this help and exit.
            \\-v, --version         Display the program version and exit.
            \\-e, --ess             Enable ESS adapter with oot-vc mapping.
            \\-m, --mapping <NAME>  Enable ESS adapter with the specified mapping. Available mappings are: oot-vc, mm-vc, z64-gc.
            \\-s, --server          Enable UDP input server on default port for gcviewer.
            \\-p, --port <PORT>     Enable UDP input server on specified port.
            \\    --calibrate       Use calibration to scale controller to full Windows range.
            \\-o, --overscale <f32> Scale control stick value by multipler. Requires --calibrate.
            \\    --oot             Read rumble data from OoT 1.0 on emulator.
            \\
        );

        const parsers = comptime .{
            .NAME = clap.parsers.string,
            .PORT = clap.parsers.int(u16, 10),
            .f32 = clap.parsers.float(f32),
        };

        var diag = clap.Diagnostic{};
        var res = clap.parse(clap.Help, &params, parsers, .{
            .diagnostic = &diag,
        }) catch |err| {
            diag.report(std.io.getStdErr().writer(), err) catch {};
            return;
        };
        defer res.deinit();

        if (res.args.help) {
            try clap.help(std.io.getStdErr().writer(), clap.Help, &params, .{});
            return;
        }

        if (res.args.version) {
            std.io.getStdErr().writer().print("{s}", .{build_info.version}) catch {};
            return;
        }

        const port = res.args.port orelse if (res.args.server)
            @as(u16, 4096)
        else
            null;

        const ess_mapping = if (res.args.mapping) |m|
            ess.Mapping.fromFileName(m) orelse {
                std.log.err("Invalid mapping specified.", .{});
                return;
            }
        else if (res.args.ess)
            ess.Mapping.oot_vc
        else
            null;

        if (!res.args.calibrate and res.args.overscale != null) {
            std.log.err("--overscale requires --calibrate.", .{});
            return;
        }

        break :blk Options{
            .ess_mapping = ess_mapping,
            .port = port,
            .use_calibration = res.args.calibrate,
            .emulator_rumble = res.args.oot,
            .overscale = res.args.overscale,
        };
    };

    std.log.info("Initializing...", .{});

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    const sock = blk: {
        if (options.port) |p| {
            const s = try std.x.os.Socket.init(
                std.os.AF.INET,
                std.os.SOCK.DGRAM,
                0,
                .{ .close_on_exec = true },
            );

            try s.connect(.{ .ipv4 = .{
                .host = std.x.os.IPv4.localhost,
                .port = p,
            } });

            std.log.info("Opened UDP server on port {}", .{p});
            break :blk s;
        } else {
            break :blk null;
        }
    };
    defer if (sock) |s| s.deinit();

    var thread_ctx = Context{
        .allocator = allocator,
        .mutex = std.Thread.Mutex{},
        .usb_ctx = &ctx,
        .sock = if (sock) |*s| s else null,
        .ess_mapping = options.ess_mapping,
        .use_calibration = options.use_calibration,
        .emulator_rumble = options.emulator_rumble,
        .overscale = options.overscale,
    };
    defer {
        if (thread_ctx.config_arena) |a| a.deinit();
        if (thread_ctx.adapter) |a| a.deinit();
        if (thread_ctx.bridge) |b| b.deinit();
    }

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

    try gui.runImGui(allocator);
}
