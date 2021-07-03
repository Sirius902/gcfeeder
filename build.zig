const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;
const LibExeObjStep = std.build.LibExeObjStep;

pub fn build(b: *Builder) void {
    // Standard target options allows the person running `zig build` to choose
    // what target to build for. Here we do not override the defaults, which
    // means any target is allowed, and the default is native. Other options
    // for restricting supported target set are available.
    const target = b.standardTargetOptions(.{});

    // Standard release options allow the person running `zig build` to select
    // between Debug, ReleaseSafe, ReleaseFast, and ReleaseSmall.
    const mode = b.standardReleaseOptions();

    const feeder_exe = feederExecutable(b);
    const viewer_exe = viewerExecutable(b);

    const dll_step = DllStep.create(b);
    b.default_step.dependOn(&dll_step.step);

    for ([_]*LibExeObjStep{ feeder_exe, viewer_exe }) |exe| {
        exe.setTarget(target);
        exe.setBuildMode(mode);
        exe.install();

        if (exe.install_step) |install_step| {
            dll_step.step.dependOn(&install_step.step);
        }
    }

    const run_cmd = feeder_exe.run();
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);
}

fn feederExecutable(b: *Builder) *LibExeObjStep {
    const exe = b.addExecutable("gcfeeder", "src/feeder/main.zig");

    exe.c_std = .C99;
    exe.addIncludeDir("include");

    exe.addLibPath("lib");

    exe.linkLibC();
    exe.linkSystemLibrary("libusb-1.0");
    exe.linkSystemLibrary("vJoyInterface");

    exe.addPackagePath("zusb", "pkg/zusb/zusb.zig");
    exe.addPackagePath("network", "pkg/zig-network/network.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");

    exe.addIncludeDir("src/feeder/ess");
    exe.addCSourceFile("src/feeder/ess/ESS.c", &[_][]const u8{});

    return exe;
}

fn viewerExecutable(b: *Builder) *LibExeObjStep {
    const exe = b.addExecutable("gcviewer", "src/viewer/main.zig");

    exe.c_std = .C99;
    exe.addIncludeDir("include");

    exe.addLibPath("lib");

    exe.linkLibC();
    exe.linkSystemLibrary("libepoxy");
    exe.linkSystemLibrary("glfw3dll");

    exe.addPackagePath("adapter", "src/feeder/adapter.zig");
    exe.addPackagePath("zgl", "pkg/zgl/zgl.zig");
    exe.addPackagePath("zglfw", "pkg/zglfw/src/main.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("network", "pkg/zig-network/network.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");

    return exe;
}

const DllStep = struct {
    step: Step,
    builder: *Builder,

    pub fn create(b: *Builder) *DllStep {
        var self = b.allocator.create(DllStep) catch unreachable;
        self.* = DllStep{
            .step = Step.init(.custom, "dll", b.allocator, make),
            .builder = b,
        };

        return self;
    }

    fn make(step: *Step) !void {
        const self = @fieldParentPtr(DllStep, "step", step);
        const b = self.builder;

        var lib = try std.fs.cwd().openDir("lib", .{ .iterate = true });
        defer lib.close();

        var exe_dir = try std.fs.cwd().openDir(b.exe_dir, .{});
        defer exe_dir.close();

        var files = lib.iterate();
        while (try files.next()) |file| {
            if (std.mem.endsWith(u8, file.name, ".dll")) {
                try lib.copyFile(file.name, exe_dir, file.name, .{});
            }
        }
    }
};
