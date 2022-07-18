const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;
const LibExeObjStep = std.build.LibExeObjStep;
const OptionsStep = std.build.OptionsStep;

pub fn build(b: *Builder) void {
    const target = b.standardTargetOptions(.{});
    const mode = b.standardReleaseOptions();

    const version_opt = b.option([]const u8, "version", "Build version string");

    const feeder_exe = feederExecutable(b);
    const viewer_exe = viewerExecutable(b);

    const dll_step = DllStep.create(b);
    b.default_step.dependOn(&dll_step.step);

    const commit_hash = getGitCommitHash(b);

    const version = version_opt orelse if (commit_hash) |h| h.short else "UNKNOWN";
    const schema_url = std.mem.join(b.allocator, "", &[_][]const u8{
        "https://raw.githubusercontent.com/Sirius902/gcfeeder/",
        if (commit_hash) |h| h.long else "main",
        "/schema/gcfeeder.schema.json",
    }) catch |err| {
        std.log.err("Failed to join schema url: {}", .{err});
        return;
    };

    const optionStep = OptionsStep.create(b);
    optionStep.addOption([]const u8, "version", version);
    optionStep.addOption([]const u8, "schema_url", schema_url);

    for ([_]*LibExeObjStep{ feeder_exe, viewer_exe }) |exe| {
        exe.setTarget(target);
        exe.setBuildMode(mode);
        exe.install();

        exe.addPackage(optionStep.getPackage("build_info"));

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

    const cxx_flags = [_][]const u8{
        "-std=c++20",
        "-fno-exceptions",
        // HACK: Define this Windows error used by the ViGEmClient because it's not
        // defined in MinGW headers.
        "-D ERROR_INVALID_DEVICE_OBJECT_PARAMETER=650L",
    };

    exe.linkLibCpp();
    exe.addCSourceFile("src/feeder/bridge/ViGEmClient/ViGEmClient.cpp", &cxx_flags);

    exe.linkSystemLibrary("libusb-1.0.dll");
    exe.linkSystemLibrary("setupapi");

    exe.addPackagePath("zusb", "pkg/zusb/zusb.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");
    exe.addPackagePath("grindel", "pkg/grindel/grindel.zig");

    return exe;
}

fn viewerExecutable(b: *Builder) *LibExeObjStep {
    const exe = b.addExecutable("gcviewer", "src/viewer/main.zig");

    exe.c_std = .C99;
    exe.addIncludeDir("include");

    exe.addLibPath("lib");

    exe.linkLibC();
    exe.linkSystemLibrary("libepoxy.dll");
    exe.linkSystemLibrary("glfw3dll");

    exe.addPackagePath("adapter", "src/feeder/adapter.zig");
    exe.addPackagePath("zgl", "pkg/zgl/zgl.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");

    return exe;
}

pub const CommitHash = struct {
    short: []const u8,
    long: []const u8,
};

fn getGitCommitHash(b: *Builder) ?CommitHash {
    const command = [_][]const u8{ "git", "rev-parse" };

    const short_ret = std.ChildProcess.exec(.{
        .allocator = b.allocator,
        .argv = command ++ &[_][]const u8{ "--short", "HEAD" },
    }) catch return null;
    const long_ret = std.ChildProcess.exec(.{
        .allocator = b.allocator,
        .argv = command ++ &[_][]const u8{"HEAD"},
    }) catch return null;

    return CommitHash{
        .short = std.mem.trim(u8, short_ret.stdout, &std.ascii.spaces),
        .long = std.mem.trim(u8, long_ret.stdout, &std.ascii.spaces),
    };
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

        var lib = try std.fs.cwd().openIterableDir("lib", .{});
        defer lib.close();

        var exe_dir = try std.fs.cwd().openDir(b.exe_dir, .{});
        defer exe_dir.close();

        var files = lib.iterate();
        while (try files.next()) |file| {
            if (std.mem.endsWith(u8, file.name, ".dll")) {
                try lib.dir.copyFile(file.name, exe_dir, file.name, .{});
            }
        }
    }
};
