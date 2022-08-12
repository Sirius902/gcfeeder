const std = @import("std");
const builtin = @import("builtin");
const build_info = @import("build_info");
const usb = @import("zusb");
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

pub const Context = struct {
    allocator: std.mem.Allocator,
    config_arena: ?std.heap.ArenaAllocator = null,
    rwl: std.Thread.RwLock = .{},
    usb_ctx: *usb.Context,
    adapter: ?Adapter = null,
    bridge: ?Bridge = null,
    reloading: Atomic(bool) = Atomic(bool).init(true),
    adapter_errored: Atomic(bool) = Atomic(bool).init(false),
    bridge_errored: Atomic(bool) = Atomic(bool).init(false),
    stop: Atomic(bool) = Atomic(bool).init(false),
    sock: ?std.x.os.Socket = null,
    ess_mapping: ?ess.Mapping = null,
    config_file: ?ConfigFile = null,
    config: ?*Config = null,
    is_calibration_bad: bool = false,
};

const fail_timeout = 100 * time.ns_per_ms;

fn loadAndSetConfig(context: *Context) !void {
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

    const config = &profile.config;

    // Free previous configuration.
    if (context.config_arena) |prev_arena| {
        prev_arena.deinit();
        context.config_arena = null;
    }

    // Invalidate outdated configuration derived structures.
    if (context.bridge) |bridge| {
        bridge.deinit();
        context.bridge = null;
    }

    if (context.sock) |sock| {
        sock.deinit();
        context.sock = null;
    }

    const sock = blk: {
        if (config.input_server.enabled) {
            const s = try std.x.os.Socket.init(
                std.os.AF.INET,
                std.os.SOCK.DGRAM,
                0,
                .{ .close_on_exec = true },
            );

            const port = config.input_server.port;
            try s.connect(.{ .ipv4 = .{
                .host = std.x.os.IPv4.localhost,
                .port = port,
            } });

            std.log.info("Opened UDP server on port {}", .{port});
            break :blk s;
        } else {
            break :blk null;
        }
    };

    context.config_arena = config_arena;
    context.config_file = config_file;
    context.config = config;
    context.sock = sock;
    context.ess_mapping = config.ess.inversion_mapping;
    context.is_calibration_bad = false;

    std.log.info("Config loaded with profile \"{s}\"", .{profile_name});
    if (config.calibration.enabled and config.calibration.data == null) {
        std.log.warn("Ignoring calibration: missing data", .{});
    }
    gui.notifyReload();
}

inline fn shouldReloadAdapter(context: *Context) bool {
    return context.adapter == null or context.adapter_errored.load(.Acquire);
}

inline fn shouldReloadBridge(context: *Context) bool {
    return context.bridge == null or context.bridge_errored.load(.Acquire);
}

fn configLoop(context: *Context) void {
    var failed = false;

    while (!context.stop.load(.Acquire)) {
        if (failed) {
            time.sleep(fail_timeout);
            failed = false;
        }

        const config_reload = context.config == null or gui.isReloadNeeded();
        const needs_reload = config_reload or shouldReloadAdapter(context) or shouldReloadBridge(context);

        if (!needs_reload) {
            time.sleep(8 * time.ns_per_ms);
            continue;
        }

        context.reloading.store(true, .Release);
        context.rwl.lock();
        defer {
            context.rwl.unlock();
            context.reloading.store(false, .Release);
        }

        if (config_reload) {
            loadAndSetConfig(context) catch |err| {
                // TODO: error to gui
                std.debug.panic("Failed to load config: {}", .{err});
            };
        }

        const config = context.config.?;

        if (shouldReloadAdapter(context)) {
            if (context.adapter) |adapter| {
                adapter.deinit();
                context.adapter = null;
                std.log.info("Disconnected from adapter", .{});
                context.adapter_errored.store(false, .Release);
            }

            const adapter = Adapter.init(context.usb_ctx) catch |err| {
                std.log.err("{} in config thread", .{err});
                failed = true;
                continue;
            };

            std.log.info("Connected to adapter", .{});
            context.adapter = adapter;
        }

        if (shouldReloadBridge(context)) {
            if (context.bridge) |bridge| {
                bridge.deinit();
                context.bridge = null;
                std.log.info("Disconnected from {s}", .{bridge.driverName()});
                context.bridge_errored.store(false, .Release);
            }

            const bridge = switch (config.driver) {
                .vigem => ViGEmBridge.initBridge(context.allocator, config.vigem_config),
            } catch |err| {
                std.log.err("{} in config thread", .{err});
                failed = true;
                continue;
            };

            std.log.info("Connected to {s}", .{bridge.driverName()});
            context.bridge = bridge;
        }
    }
}

fn inputLoop(context: *Context) void {
    var failed = false;

    while (!context.stop.load(.Acquire)) {
        if (context.reloading.load(.Acquire) or context.adapter == null or context.bridge == null) {
            time.sleep(8 * time.ns_per_ms);
            failed = false;
            continue;
        } else if (failed) {
            time.sleep(fail_timeout);
            failed = false;
            continue;
        }

        context.rwl.lockShared();
        defer context.rwl.unlockShared();

        const adapter = &context.adapter.?;
        const bridge = context.bridge.?;
        const config = context.config.?;

        const inputs = adapter.readInputs() catch |err| {
            switch (err) {
                error.Timeout => continue,
                else => {
                    context.adapter_errored.store(true, .Release);
                    std.log.err("{} in input thread", .{err});
                    failed = true;
                    continue;
                },
            }
        };

        if (inputs[0]) |input| {
            const ess_mapped = if (context.ess_mapping) |m| ess.map(m, input) else input;
            const calibrated = if (config.calibration.enabled and config.calibration.data != null and !context.is_calibration_bad)
                config.calibration.data.?.map(ess_mapped) catch {
                    context.is_calibration_bad = true;
                    std.log.warn("Ignoring calibration: bad calibration data", .{});
                    continue;
                }
            else
                ess_mapped;

            const should_apply_scaling = !std.math.approxEqAbs(f32, config.analog_scale, 1.0, 1e-5);
            const scaled = if (should_apply_scaling)
                calibrate.applyScaling(calibrated, config.analog_scale)
            else
                calibrated;

            gui.updateInputs(.{
                .main_stick = .{
                    .raw = .{ .x = input.stick_x, .y = input.stick_y },
                    .mapped = .{ .x = ess_mapped.stick_x, .y = ess_mapped.stick_y },
                    .calibrated = .{ .x = calibrated.stick_x, .y = calibrated.stick_y },
                    .scaled = .{ .x = scaled.stick_x, .y = scaled.stick_y },
                },
                .c_stick = .{
                    .raw = .{ .x = input.substick_x, .y = input.substick_y },
                    .mapped = .{ .x = ess_mapped.substick_x, .y = ess_mapped.substick_y },
                    .calibrated = .{ .x = calibrated.substick_x, .y = calibrated.substick_y },
                    .scaled = .{ .x = scaled.substick_x, .y = scaled.substick_y },
                },
                .a_pressed = @boolToInt(input.button_a),
                .active_stages = blk: {
                    var s = gui.Stage.raw;
                    if (context.ess_mapping != null) s |= gui.Stage.mapped;
                    if (config.calibration.enabled and config.calibration.data != null and !context.is_calibration_bad) s |= gui.Stage.calibrated;
                    if (should_apply_scaling) s |= gui.Stage.scaled;
                    break :blk s;
                },
            });

            const to_feed = if (gui.isCalibrating()) Input.default else calibrated;

            bridge.feed(to_feed) catch |err| {
                context.bridge_errored.store(true, .Release);
                std.log.err("{} in input thread", .{err});
                failed = true;
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
    var failed = false;
    var rumble = Rumble.Off;

    var handle: ?emulator.Handle = null;
    defer if (handle) |h| h.close();

    while (!context.stop.load(.Acquire)) {
        if (context.reloading.load(.Acquire) or context.adapter == null or context.bridge == null) {
            time.sleep(8 * time.ns_per_ms);
            failed = false;
            continue;
        } else if (failed) {
            time.sleep(fail_timeout);
            failed = false;
            continue;
        }

        context.rwl.lockShared();
        defer context.rwl.unlockShared();

        const adapter = &context.adapter.?;
        const bridge = context.bridge.?;
        const config = context.config.?;

        switch (config.rumble) {
            .off => {
                rumble = .Off;
            },
            .on => {
                if (bridge.pollRumble()) |r| {
                    rumble = r;
                }
            },
            .emulator => {
                if (handle) |h| {
                    rumble = h.rumbleState() catch blk: {
                        std.log.info("Disconnected from {s}", .{h.emulatorTitle()});
                        h.close();
                        handle = null;
                        break :blk .Off;
                    };
                } else {
                    handle = emulator.Handle.open() catch blk: {
                        failed = true;
                        break :blk null;
                    };
                    if (handle) |h| std.log.info("Connected to {s} OoT 1.0", .{h.emulatorTitle()});
                    rumble = .Off;
                }
            },
        }

        adapter.setRumble(.{ rumble, .Off, .Off, .Off }) catch |err| {
            switch (err) {
                error.Timeout => continue,
                else => {
                    context.adapter_errored.store(true, .Release);
                    std.log.err("{} in rumble thread", .{err});
                    failed = true;
                    continue;
                },
            }
        };
    }

    if (context.adapter) |adapter| {
        adapter.setRumble(.{ .Off, .Off, .Off, .Off }) catch {};
    }
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    gui.log_allocator = allocator;

    std.log.info("Initializing...", .{});

    var ctx = try usb.Context.init();
    defer ctx.deinit();

    var thread_ctx = Context{
        .allocator = allocator,
        .usb_ctx = &ctx,
    };
    defer {
        if (thread_ctx.config_arena) |a| a.deinit();
        if (thread_ctx.adapter) |a| a.deinit();
        if (thread_ctx.bridge) |b| b.deinit();
        if (thread_ctx.sock) |s| s.deinit();
    }

    var threads = [_]std.Thread{
        try std.Thread.spawn(.{}, configLoop, .{&thread_ctx}),
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
