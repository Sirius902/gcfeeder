const std = @import("std");
const time = std.time;
const Thread = std.Thread;
const AtomicBool = @import("atomic.zig").Bool;
const print = std.debug.print;

fn doot(stop: *AtomicBool) void {
    while (!stop.get()) {
        print("doot\n", .{});
        time.sleep(time.ns_per_s / 4);
    }
}

pub fn main() !void {
    var stop = AtomicBool.init(false);
    const thread1 = try Thread.spawn(&stop, doot);

    time.sleep(time.ns_per_s * 2);

    stop.set(true);
    thread1.wait();
}
