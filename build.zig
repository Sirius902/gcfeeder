const std = @import("std");
const Builder = std.build.Builder;
const Step = std.build.Step;

pub fn build(b: *Builder) void {
    const version_opt = b.option([]const u8, "version", "Override build version string");
    const no_git = b.option(bool, "no-git", "Do not use Git to obtain build info");

    const zig_fmt = b.addFmt(&[_][]const u8{"build.zig"});
    const fmt_step = b.step("fmt", "Format source excluding pkg and external");
    fmt_step.dependOn(&zig_fmt.step);

    const options = blk: {
        var opt = std.ArrayList([]const u8).init(b.allocator);
        if (version_opt) |v| {
            opt.append(std.mem.concat(
                b.allocator,
                u8,
                &[_][]const u8{ "-Dversion=", v },
            ) catch unreachable) catch unreachable;
        }
        if (no_git) |ng| {
            const s = if (ng) "true" else "false";
            opt.append(std.mem.concat(
                b.allocator,
                u8,
                &[_][]const u8{ "-Dno-git=", s },
            ) catch unreachable) catch unreachable;
        }
        break :blk opt.toOwnedSlice();
    };

    inline for (.{ "feeder", "viewer" }) |name| {
        var zig_sub_build_step = BuildStep.create(b, name, null, options);
        var zig_sub_run_step = BuildStep.create(b, name, "run", options);
        var zig_sub_fmt_step = BuildStep.create(b, name, "fmt", options);

        var sub_build_step = b.step("build-" ++ name, "Build gc" ++ name);
        sub_build_step.dependOn(&zig_sub_build_step.step);
        b.default_step.dependOn(sub_build_step);

        var sub_run_step = b.step("run-" ++ name, "Run gc" ++ name);
        sub_run_step.dependOn(&zig_sub_run_step.step);

        var sub_fmt_step = b.step("fmt-" ++ name, "Format gc" ++ name);
        sub_fmt_step.dependOn(&zig_sub_fmt_step.step);
        fmt_step.dependOn(sub_fmt_step);
    }
}

pub const BuildStep = struct {
    step: Step,
    builder: *Builder,
    cwd: []const u8,
    argv: [][]const u8,

    pub fn create(builder: *Builder, cwd: []const u8, command: ?[]const u8, args: []const []const u8) *BuildStep {
        const self = builder.allocator.create(BuildStep) catch unreachable;
        const name = "zig build";
        const argc_extra: usize = if (command == null) 2 else 3;
        self.* = BuildStep{
            .step = Step.init(.custom, name, builder.allocator, make),
            .builder = builder,
            .cwd = cwd,
            .argv = builder.allocator.alloc([]u8, args.len + argc_extra) catch unreachable,
        };

        self.argv[0] = builder.zig_exe;
        self.argv[1] = "build";
        if (command) |cmd| {
            self.argv[2] = cmd;
        }
        for (args) |arg, i| {
            self.argv[argc_extra + i] = arg;
        }
        return self;
    }

    fn make(step: *Step) !void {
        const self = @fieldParentPtr(BuildStep, "step", step);

        return self.builder.spawnChildEnvMap(self.cwd, self.builder.env_map, self.argv);
    }
};
