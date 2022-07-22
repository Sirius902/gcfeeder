const std = @import("std");
const build_info = @import("build_info");
const clap = @import("clap");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;
const display = @import("display.zig");
const Atomic = std.atomic.Atomic;

pub const log_level = .info;
pub const user_shader_path = "color.frag";

pub const Context = struct {
    mutex: std.Thread.Mutex,
    allocator: std.mem.Allocator,
    sock: *const std.x.os.Socket,
    input: ?Input,
    stop: Atomic(bool),
};

fn receiveLoop(context: *Context) !void {
    while (!context.stop.load(.Acquire)) {
        var buffer: [@sizeOf(Input)]u8 = undefined;

        const data_len = context.sock.read(&buffer, 0) catch |err| {
            switch (err) {
                error.WouldBlock => {
                    std.time.sleep(8 * std.time.ns_per_ms);
                    continue;
                },
                else => return err,
            }
        };

        if (data_len == buffer.len) {
            context.mutex.lock();
            defer context.mutex.unlock();

            context.input = Input.deserialize(&buffer);
        } else {
            std.log.err("Socket received incomplete data of size {}", .{data_len});
            break;
        }
    }
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const sock = try std.x.os.Socket.init(
        std.os.AF.INET,
        std.os.SOCK.DGRAM,
        0,
        .{ .nonblocking = true, .close_on_exec = true },
    );

    const port = blk: {
        const params = comptime clap.parseParamsComptime(
            \\-h, --help        Display this help and exit.
            \\-v, --version     Display the program version and exit.
            \\-p, --port <PORT> Listen to UDP input server on specified port.
            \\
        );

        const parsers = comptime .{
            .MAP = clap.parsers.string,
            .PORT = clap.parsers.int(u16, 10),
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

        break :blk res.args.port orelse 4096;
    };

    try sock.bind(.{ .ipv4 = .{
        .host = std.x.os.IPv4.localhost,
        .port = port,
    } });

    var context = Context{
        .mutex = std.Thread.Mutex{},
        .allocator = allocator,
        .sock = &sock,
        .input = null,
        .stop = Atomic(bool).init(false),
    };

    const thread = try std.Thread.spawn(.{}, receiveLoop, .{&context});
    defer {
        context.stop.store(true, .Release);
        thread.join();
        sock.deinit();
    }

    std.log.info("Listening on UDP port {}", .{port});

    const color_shader_source: ?[]const u8 = blk: {
        const exe_dir_path = std.fs.selfExeDirPathAlloc(allocator) catch break :blk null;
        defer allocator.free(exe_dir_path);

        var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
        defer exe_dir.close();

        const file = exe_dir.openFile(user_shader_path, .{}) catch break :blk null;
        defer file.close();

        break :blk try file.readToEndAlloc(allocator, std.math.maxInt(usize));
    };
    defer if (color_shader_source) |cs| allocator.free(cs);

    try display.show(&context, color_shader_source);
}
