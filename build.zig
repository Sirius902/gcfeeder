const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;
const LibExeObjStep = std.build.LibExeObjStep;
const OptionsStep = std.build.OptionsStep;
const FmtStep = std.build.FmtStep;

pub fn build(b: *Builder) void {
    const target = b.standardTargetOptions(.{});
    const mode = b.standardReleaseOptions();

    const version_opt = b.option([]const u8, "version", "Override build version string");
    const no_git = b.option(bool, "no-git", "Do not use Git to obtain build info") orelse false;

    const build_info = if (no_git)
        BuildInfo{ .version = "unknown", .usercontent_ref = "main" }
    else
        getBuildInfoFromGit(b.allocator) catch |err| {
            std.log.err("Failed to get build info from Git: {}", .{err});
            return;
        };

    const usercontent_url = std.mem.join(b.allocator, "/", &[_][]const u8{
        "https://raw.githubusercontent.com/Sirius902/gcfeeder",
        build_info.usercontent_ref,
    }) catch |err| {
        std.log.err("Failed to join usercontent url: {}", .{err});
        return;
    };

    const options = OptionsStep.create(b);
    options.addOption([]const u8, "version", version_opt orelse build_info.version);
    options.addOption([]const u8, "usercontent_url", usercontent_url);

    const params = .{ .b = b, .target = target, .mode = mode, .options = options };
    const feeder_exe = addFeederExecutable(params);

    const viewer_exe = addViewerExecutable(params);

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

    const zig_fmt = FmtStep.create(b, &[_][]const u8{ "build.zig", "src" });
    const clang_format = b.addSystemCommand(&[_][]const u8{ "clang-format", "-i", "-style=file" } ++ feeder_cxx_all);
    const fmt_step = b.step("fmt", "Format source excluding include and pkg");
    fmt_step.dependOn(&zig_fmt.step);
    fmt_step.dependOn(&clang_format.step);
}

const BuildParams = struct {
    b: *Builder,
    target: std.zig.CrossTarget,
    mode: std.builtin.Mode,
    options: *OptionsStep,
};

const feeder_cxx_header = [_][]const u8{
    "src/feeder/gui/cpp/gui.h",
    "src/feeder/gui/cpp/app_log.h",
};

const feeder_cxx_source = [_][]const u8{
    "src/feeder/gui/cpp/gui.cpp",
    "src/feeder/gui/cpp/app_log.cpp",
};

const feeder_cxx_all = feeder_cxx_header ++ feeder_cxx_source;

fn addFeederExecutable(params: BuildParams) *LibExeObjStep {
    const exe = params.b.addExecutable("gcfeeder", "src/feeder/main.zig");
    const dll_deps = .{
        .{ .lib = "usb-1.0.dll", .dll = "libusb-1.0.dll" },
        .{ .lib = "epoxy.dll", .dll = "libepoxy-0.dll" },
        .{ .lib = "glfw3dll", .dll = "glfw3.dll" },
    };

    exe.addIncludeDir("include");
    exe.addIncludeDir("src/feeder/gui/cpp");
    exe.addLibPath("lib");

    exe.linkLibCpp();
    exe.linkSystemLibrary("setupapi");

    inline for (dll_deps) |dep| {
        exe.linkSystemLibrary(dep.lib);
    }

    const vigem_cxx_flags = [_][]const u8{
        "-std=c++20",
        // HACK: Define this Windows error used by the ViGEmClient because it's not
        // defined in MinGW headers.
        "-D ERROR_INVALID_DEVICE_OBJECT_PARAMETER=650L",
    };

    exe.addCSourceFile("include/ViGEmClient/ViGEmClient.cpp", &vigem_cxx_flags);

    const cxx_flags = [_][]const u8{
        "-std=c++20",
        "-Wpedantic",
        "-Werror",
        "-Wall",
        "-Wextra",
        "-Iinclude/imgui",
    };

    const include_cxx_source = [_][]const u8{
        "include/imgui/imgui_demo.cpp",
        "include/imgui/imgui_draw.cpp",
        "include/imgui/imgui_tables.cpp",
        "include/imgui/imgui_widgets.cpp",
        "include/imgui/imgui.cpp",
        "include/imgui/backends/imgui_impl_glfw.cpp",
        "include/imgui/backends/imgui_impl_opengl3.cpp",
    };

    exe.addCSourceFiles(&include_cxx_source ++ feeder_cxx_source, &cxx_flags);

    exe.addPackagePath("zusb", "pkg/zusb/zusb.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");
    exe.addPackagePath("grindel", "pkg/grindel/grindel.zig");

    exe.addPackage(params.options.getPackage("build_info"));

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
        .{ .lib = "epoxy.dll", .dll = "libepoxy-0.dll" },
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

    exe.addPackage(params.options.getPackage("build_info"));

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

const BuildInfo = struct {
    version: []const u8,
    usercontent_ref: []const u8,
};

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
