const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;
const LibExeObjStep = std.build.LibExeObjStep;
const OptionsStep = std.build.OptionsStep;

const cxx_flags = [_][]const u8{
    "-std=c++20",
    "-Wpedantic",
    "-Werror",
    "-Wall",
    "-Wextra",
};

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

    const exe = b.addExecutable("gcfeeder", "src/main.zig");
    exe.setTarget(target);
    exe.setBuildMode(mode);
    exe.install();

    exe.linkLibCpp();
    linkLibusb(b, exe);
    linkVigem(exe);
    linkGlad(exe);
    linkGlfw(b, exe);
    linkImGui(exe);

    const cxx_header = [_][]const u8{
        "src/gui/cpp/gui.h",
        "src/gui/cpp/app_log.h",
    };

    const cxx_source = [_][]const u8{
        "src/gui/cpp/gui.cpp",
        "src/gui/cpp/app_log.cpp",
    };

    const cxx_all = cxx_header ++ cxx_source;

    exe.addIncludePath("src/gui/cpp");
    exe.addCSourceFiles(&cxx_source, &cxx_flags);

    exe.addPackagePath("zusb", "pkg/zusb/zusb.zig");
    exe.addPackagePath("zlm", "pkg/zlm/zlm.zig");
    exe.addPackagePath("clap", "pkg/zig-clap/clap.zig");
    exe.addPackagePath("grindel", "pkg/grindel/grindel.zig");
    build_info.addPackageTo(exe, "build_info");

    const run_cmd = exe.run();
    run_cmd.step.dependOn(b.getInstallStep());
    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    const zig_fmt = b.addFmt(&[_][]const u8{ "build.zig", "src" });
    const clang_format = b.addSystemCommand(&[_][]const u8{ "clang-format", "-i", "-style=file" } ++ cxx_all);
    const fmt_step = b.step("fmt", "Format source excluding pkg and external");
    fmt_step.dependOn(&zig_fmt.step);
    fmt_step.dependOn(&clang_format.step);
}

fn linkLibusb(b: *Builder, lib_exe: *LibExeObjStep) void {
    const sub_root = "external/libusb-1.0";
    const dll_name = "libusb-1.0.dll";
    const dll_path = sub_root ++ "/bin/" ++ dll_name;
    lib_exe.addIncludePath(sub_root ++ "/include");
    lib_exe.addLibraryPath(sub_root ++ "/lib");

    lib_exe.linkSystemLibrary("usb-1.0.dll");

    if (lib_exe.install_step) |install_step| {
        const install_dll = b.addInstallFileWithDir(.{ .path = dll_path }, .bin, dll_name);
        install_step.step.dependOn(&install_dll.step);
    }
}

fn linkVigem(lib_exe: *LibExeObjStep) void {
    const sub_root = "external/ViGEm";
    lib_exe.addIncludeDir(sub_root ++ "/include");

    lib_exe.linkSystemLibrary("setupapi");

    const vigem_cxx_flags = [_][]const u8{
        "-std=c++20",
        // HACK: Define this Windows error used by the ViGEmClient because it's not
        // defined in MinGW headers.
        "-D ERROR_INVALID_DEVICE_OBJECT_PARAMETER=650L",
    };

    lib_exe.addCSourceFile(sub_root ++ "/src/ViGEmClient/ViGEmClient.cpp", &vigem_cxx_flags);
}

fn linkGlad(lib_exe: *LibExeObjStep) void {
    const sub_root = "external/glad";
    lib_exe.addIncludeDir(sub_root ++ "/include");

    lib_exe.addCSourceFile(sub_root ++ "/src/glad.c", &[_][]const u8{"-std=c17"});
}

fn linkGlfw(b: *Builder, lib_exe: *LibExeObjStep) void {
    const sub_root = "external/GLFW";
    const dll_name = "glfw3.dll";
    const dll_path = sub_root ++ "/bin/" ++ dll_name;
    lib_exe.addIncludeDir(sub_root ++ "/include");
    lib_exe.addLibraryPath(sub_root ++ "/lib");

    lib_exe.linkSystemLibrary("glfw3dll");

    if (lib_exe.install_step) |install_step| {
        const install_dll = b.addInstallFileWithDir(.{ .path = dll_path }, .bin, dll_name);
        install_step.step.dependOn(&install_dll.step);
    }
}

fn linkImGui(lib_exe: *LibExeObjStep) void {
    const sub_root = "external/imgui";
    lib_exe.addIncludeDir(sub_root);

    const imgui_cxx_source = [_][]const u8{
        sub_root ++ "/imgui_demo.cpp",
        sub_root ++ "/imgui_draw.cpp",
        sub_root ++ "/imgui_tables.cpp",
        sub_root ++ "/imgui_widgets.cpp",
        sub_root ++ "/imgui.cpp",
        sub_root ++ "/backends/imgui_impl_glfw.cpp",
        sub_root ++ "/backends/imgui_impl_opengl3.cpp",
    };

    const imgui_cxx_flags = cxx_flags ++ [_][]const u8{"-Iinclude/imgui"};

    lib_exe.addCSourceFiles(&imgui_cxx_source, &imgui_cxx_flags);
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
            .options = builder.addOptions(),
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
