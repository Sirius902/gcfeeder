const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;

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
        std.debug.warn("{} error while trying to stringify the target triple\n", .{err});
        std.os.exit(1);
    };

    const lib_dir = std.fs.path.join(b.allocator, &[_][]const u8{ "lib", target_triple_str }) catch |err| {
        std.debug.warn("{} error while trying to render library path\n", .{err});
        std.os.exit(1);
    };

    exe.addLibPath(lib_dir);

    exe.linkLibC();
    exe.linkSystemLibrary("libusb-1.0");
    exe.linkSystemLibrary("vJoyInterface");
    exe.linkSystemLibrary("glfw3dll");

    exe.addIncludeDir("src/ess");
    exe.addCSourceFile("src/ess/ESS.c", &[_][]const u8{});

    exe.setTarget(target);
    exe.setBuildMode(mode);
    exe.install();

    if (exe.install_step) |install_step| {
        const dll_step = DllStep.create(b, target_triple_str);
        dll_step.step.dependOn(&install_step.step);
        b.default_step.dependOn(&dll_step.step);
    }

    const run_cmd = exe.run();
    run_cmd.step.dependOn(b.getInstallStep());

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);
}

const DllStep = struct {
    step: Step,
    builder: *Builder,
    target_triple_str: []const u8,

    pub fn create(b: *Builder, target_triple_str: []const u8) *DllStep {
        var self = b.allocator.create(DllStep) catch unreachable;
        self.* = DllStep{
            .step = Step.init(.Custom, "dll", b.allocator, make),
            .builder = b,
            .target_triple_str = target_triple_str,
        };

        return self;
    }

    fn make(step: *Step) !void {
        const self = @fieldParentPtr(DllStep, "step", step);
        const b = self.builder;

        var lib = try std.fs.cwd().openDir("lib", .{});
        defer lib.close();

        var triple_lib = try lib.openDir(self.target_triple_str, .{ .iterate = true });
        defer triple_lib.close();

        var exe_dir = try std.fs.cwd().openDir(b.exe_dir, .{});
        defer exe_dir.close();

        var files = triple_lib.iterate();
        while (try files.next()) |file| {
            if (std.mem.endsWith(u8, file.name, ".dll")) {
                try triple_lib.copyFile(file.name, exe_dir, file.name, .{});
            }
        }
    }
};
