const std = @import("std");
const time = std.time;
const print = std.debug.print;

const c = @import("c.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var ctx: ?*c.libusb_context = null;
    _ = c.libusb_init(&ctx);
    defer c.libusb_exit(ctx);

    print("Hello World!\n", .{});
    print("libusb_context: {p}\n", .{ctx});
    print("vJoyEnabled: {}\n", .{c.vJoyEnabled()});

    const screen_width = 800;
    const screen_height = 640;

    c.SetTraceLogLevel(c.LOG_NONE);
    c.InitWindow(screen_width, screen_height, "gcfeeder");
    c.SetTargetFPS(60);

    var buffer = std.ArrayList(u8).init(allocator);
    defer buffer.deinit();

    // Add null-terminator.
    try buffer.append(0);

    var gen = std.rand.Xoroshiro128.init(@bitCast(u64, @truncate(i64, std.time.nanoTimestamp())));
    const rand = &gen.random;

    while (!c.WindowShouldClose()) {
        c.BeginDrawing();
        defer c.EndDrawing();

        if (c.IsKeyDown(c.KEY_UP)) {
            const letter = rand.intRangeLessThan(u8, 0, 26);
            const base: u8 = if (rand.boolean()) 'a' else 'A';
            try buffer.insert(buffer.items.len - 1, letter + base);
        }

        c.ClearBackground(c.DARKGRAY);
        c.DrawText(buffer.items.ptr, 190, 200, 20, c.LIGHTGRAY);
    }

    c.CloseWindow();
}
