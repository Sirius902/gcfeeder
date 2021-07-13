const std = @import("std");
const clap = @import("clap");
const network = @import("network");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;
const display = @import("display.zig");

pub const log_level = .info;
pub const user_shader_path = "color.frag";

pub const Context = struct {
    mutex: std.Thread.Mutex,
    allocator: *std.mem.Allocator,
    sock: *const network.Socket,
    input: ?Input,
};

fn receiveLoop(context: *Context) !void {
    while (true) {
        var buffer: [@sizeOf(Input)]u8 = undefined;

        const data_len = context.sock.receive(&buffer) catch |err| {
            switch (err) {
                error.FileDescriptorNotASocket => return,
                else => return err,
            }
        };

        if (data_len == buffer.len) {
            const held = context.mutex.acquire();
            defer held.release();

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
    const allocator = &gpa.allocator;

    try network.init();
    defer network.deinit();

    var sock = try network.Socket.create(.ipv4, .udp);

    const port = blk: {
        const params = comptime [_]clap.Param(clap.Help){
            clap.parseParam("-h, --help        Display this help and exit.        ") catch unreachable,
            clap.parseParam("-p, --port <PORT> Listen to UDP input server on port.") catch unreachable,
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

        break :blk if (args.option("--port")) |p|
            std.fmt.parseUnsigned(u16, p, 10) catch {
                std.log.err("Invalid port specified.", .{});
                return;
            }
        else
            4096;
    };

    try sock.bindToPort(port);

    var context = Context{
        .mutex = std.Thread.Mutex{},
        .allocator = allocator,
        .sock = &sock,
        .input = null,
    };

    const thread = try std.Thread.spawn(.{}, receiveLoop, .{&context});
    defer {
        sock.close();
        thread.join();
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
