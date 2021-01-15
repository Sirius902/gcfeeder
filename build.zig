const std = @import("std");
const Builder = std.build.Builder;

pub fn build(b: *Builder) void {
    // Standard target options allows the person running `zig build` to choose
    // what target to build for. Here we do not override the defaults, which
    // means any target is allowed, and the default is native. Other options
    // for restricting supported target set are available.
    const target = b.standardTargetOptions(.{});

    // Standard release options allow the person running `zig build` to select
    // between Debug, ReleaseSafe, ReleaseFast, and ReleaseSmall.
    const mode = b.standardReleaseOptions();

    const exe = b.addExecutable("gcfeeder", "src/main.zig");

    exe.c_std = .C99;
    exe.addIncludeDir("include");

    const target_triple_str = target.linuxTriple(b.allocator) catch |err| {
        std.debug.warn("{} error while trying to stringify the target triple", .{err});
        std.os.exit(1);
    };

    const lib_dir = std.fs.path.join(b.allocator, &[_][]const u8{ "lib", target_triple_str }) catch |err| {
        std.debug.warn("{} error while trying to render library path", .{err});
        std.os.exit(1);
    };

    exe.addLibPath(lib_dir);

    exe.linkLibC();
    exe.linkSystemLibrary("raylibdll");
    exe.linkSystemLibrary("libusb-1.0");
    exe.linkSystemLibrary("vJoyInterface");

    exe.setTarget(target);
    exe.setBuildMode(mode);
    exe.install();

    copyDlls(b, target_triple_str) catch |err| {
        std.debug.warn("{} error while trying to copy dlls", .{err});
        std.os.exit(1);
    };

    const run_cmd = exe.run();
    run_cmd.step.dependOn(b.getInstallStep());

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);
}

fn copyDlls(b: *Builder, target_triple_str: []const u8) !void {
    var lib = try std.fs.cwd().openDir("lib", .{});
    defer lib.close();

    var triple_lib = try lib.openDir(target_triple_str, .{ .iterate = true });
    defer triple_lib.close();

    var exe_dir = try std.fs.cwd().openDir(b.exe_dir, .{});
    defer exe_dir.close();

    var files = triple_lib.iterate();
    while (try files.next()) |file| {
        const extension = fileExtension(file.name) orelse continue;
        if (std.mem.eql(u8, extension, ".dll")) {
            try triple_lib.copyFile(file.name, exe_dir, file.name, .{});
        }
    }
}

fn fileExtension(name: []const u8) ?[]const u8 {
    return if (std.mem.lastIndexOf(u8, name, ".")) |i| name[i..] else null;
}
