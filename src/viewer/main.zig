const std = @import("std");
const network = @import("network");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;
const display = @import("display.zig");

const port = 4096;

pub const Context = struct {
    mutex: std.Thread.Mutex,
    sock: *const network.Socket,
    input: ?Input,
};

fn recieveLoop(context: *Context) !void {
    while (true) {
        var buffer: [@sizeOf(Input)]u8 = undefined;

        if ((try context.sock.receive(&buffer)) == buffer.len) {
            const held = context.mutex.acquire();
            defer held.release();

            context.input = Input.deserialize(&buffer);
        }
    }
}

pub fn main() !void {
    try network.init();
    defer network.deinit();

    var sock = try network.Socket.create(.ipv4, .udp);
    defer sock.close();

    try sock.bindToPort(port);

    var context = Context{
        .mutex = std.Thread.Mutex{},
        .sock = &sock,
        .input = null,
    };

    // TODO: Don't leak thread.
    _ = try std.Thread.spawn(recieveLoop, &context);
    // defer thread.wait();

    std.log.info("Listening on port {}", .{port});
    try display.show(&context);
}
