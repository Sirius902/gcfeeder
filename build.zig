const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;
const LibExeObjStep = std.build.LibExeObjStep;
const OptionsStep = std.build.OptionsStep;

pub fn build(b: *Builder) void {
    const target = b.standardTargetOptions(.{});
    const mode = b.standardReleaseOptions();

    const version_opt = b.option([]const u8, "version", "Override build version string");
    const no_git = b.option(bool, "no-git", "Do not use Git to obtain build info") orelse false;

    const build_info = BuildInfoStep.create(b, .{
        .no_git = no_git,
        .version_override = version_opt,
        .usercontent_root = "https://raw.githubusercontent.com/Sirius902/gcfeeder",
        .default_build_info = .{ .version = "unknown", .usercontent_ref = "main" },
    });

    const params = .{ .b = b, .target = target, .mode = mode };
    const feeder_exe = addFeederExecutable(params);
    build_info.addPackageTo(feeder_exe, "build_info");

    const viewer_exe = addViewerExecutable(params);
    build_info.addPackageTo(viewer_exe, "build_info");

    const run_feeder_cmd = feeder_exe.run();
    run_feeder_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_feeder_cmd.addArgs(args);
    }

    const run_feeder_step = b.step("run-feeder", "Run gcfeeder");
    run_feeder_step.dependOn(&run_feeder_cmd.step);

    const run_viewer_cmd = viewer_exe.run();
    run_viewer_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_viewer_cmd.addArgs(args);
    }

    const run_viewer_step = b.step("run-viewer", "Run gcviewer");
    run_viewer_step.dependOn(&run_viewer_cmd.step);

    const zig_fmt = b.addSystemCommand(&[_][]const u8{ "zig", "fmt", "build.zig", "src" });
    const fmt_step = b.step("fmt", "Format source excluding include and pkg");
    fmt_step.dependOn(&zig_fmt.step);
}

const BuildParams = struct {
    b: *Builder,
    target: std.zig.CrossTarget,
    mode: std.builtin.Mode,
};

fn addFeederExecutable(params: BuildParams) *LibExeObjStep {
    const exe = params.b.addExecutable("gcfeeder", "src/feeder/main.zig");
    const dll_deps = .{.{ .lib = "libusb-1.0.dll", .dll = "libusb-1.0.dll" }};

    exe.addIncludeDir("include");
    exe.addLibPath("lib");

    exe.linkLibCpp();
    exe.linkSystemLibrary("setupapi");

    inline for (dll_deps) |dep| {
        exe.linkSystemLibrary(dep.lib);
    }

    const cxx_flags = [_][]const u8{
        "-std=c++20",
        "-fno-exceptions",
        // HACK: Define this Windows error used by the ViGEmClient because it's not
        // defined in MinGW headers.
        "-D ERROR_INVALID_DEVICE_OBJECT_PARAMETER=650L",
    };

    exe.addCSourceFile("src/feeder/bridge/ViGEmClient/ViGEmClient.cpp", &cxx_flags);

    exe.addPackagePath("zusb", "pkg/zusb/zusb.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");
    exe.addPackagePath("grindel", "pkg/grindel/grindel.zig");

    exe.setTarget(params.target);
    exe.setBuildMode(params.mode);
    exe.install();

    if (exe.install_step) |install_step| {
        inline for (dll_deps) |dep| {
            const install_dll = params.b.addInstallFileWithDir(.{ .path = "lib/" ++ dep.dll }, .bin, dep.dll);
            install_step.step.dependOn(&install_dll.step);
        }
    }

    return exe;
}

fn addViewerExecutable(params: BuildParams) *LibExeObjStep {
    const exe = params.b.addExecutable("gcviewer", "src/viewer/main.zig");
    const dll_deps = .{
        .{ .lib = "libepoxy.dll", .dll = "libepoxy-0.dll" },
        .{ .lib = "glfw3dll", .dll = "glfw3.dll" },
    };

    exe.addIncludeDir("include");
    exe.addLibPath("lib");

    exe.linkLibC();

    inline for (dll_deps) |dep| {
        exe.linkSystemLibrary(dep.lib);
    }

    exe.addPackagePath("adapter", "src/feeder/adapter.zig");
    exe.addPackagePath("zgl", "pkg/zgl/zgl.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");

    exe.setTarget(params.target);
    exe.setBuildMode(params.mode);
    exe.install();

    if (exe.install_step) |install_step| {
        inline for (dll_deps) |dep| {
            const install_dll = params.b.addInstallFileWithDir(.{ .path = "lib/" ++ dep.dll }, .bin, dep.dll);
            install_step.step.dependOn(&install_dll.step);
        }
    }

    return exe;
}

const BuildInfoStep = struct {
    step: Step,
    options: *OptionsStep,
    builder: *Builder,
    config: Config,

    pub const BuildInfo = struct {
        version: []const u8,
        usercontent_ref: []const u8,
    };

    pub const Config = struct {
        no_git: bool = false,
        version_override: ?[]const u8 = null,
        usercontent_root: []const u8,
        default_build_info: BuildInfo,
    };

    pub fn create(builder: *Builder, config: Config) *BuildInfoStep {
        const self = builder.allocator.create(BuildInfoStep) catch unreachable;
        self.* = .{
            .builder = builder,
            .step = Step.init(.custom, "BuildInfo", builder.allocator, make),
            .options = OptionsStep.create(builder),
            .config = config,
        };

        self.options.step.dependOn(&self.step);

        return self;
    }

    pub fn addPackageTo(self: *BuildInfoStep, lib_exe: *LibExeObjStep, package_name: []const u8) void {
        lib_exe.addPackage(self.options.getPackage(package_name));
    }

    fn make(step: *Step) !void {
        const self = @fieldParentPtr(BuildInfoStep, "step", step);

        const build_info = if (self.config.no_git)
            self.config.default_build_info
        else
            getBuildInfoFromGit(self.builder.allocator) catch |err| {
                std.debug.panic("Failed to get build info from Git: {}", .{err});
                return;
            };

        const usercontent_url = std.mem.join(self.builder.allocator, "/", &[_][]const u8{
            self.config.usercontent_root,
            build_info.usercontent_ref,
        }) catch unreachable;

        self.options.addOption([]const u8, "version", self.config.version_override orelse build_info.version);
        self.options.addOption([]const u8, "usercontent_url", usercontent_url);
    }

    fn getBuildInfoFromGit(allocator: std.mem.Allocator) !BuildInfo {
        const version = try execGetStdOut(allocator, &[_][]const u8{
            "git",
            "describe",
            "--always",
            "--dirty",
            "--tags",
        });

        const commit_hash = try execGetStdOut(allocator, &[_][]const u8{
            "git",
            "rev-parse",
            "HEAD",
        });

        return BuildInfo{
            .version = std.mem.trim(u8, version, &std.ascii.spaces),
            .usercontent_ref = std.mem.trim(u8, commit_hash, &std.ascii.spaces),
        };
    }

    fn execGetStdOut(allocator: std.mem.Allocator, argv: []const []const u8) ![]const u8 {
        const result = try std.ChildProcess.exec(.{
            .allocator = allocator,
            .argv = argv,
        });

        switch (result.term) {
            .Exited => |code| {
                if (code == 0) {
                    return result.stdout;
                }
            },
            else => {},
        }

        return error.ExecFail;
    }
};
