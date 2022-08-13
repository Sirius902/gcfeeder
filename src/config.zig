const std = @import("std");
const build_info = @import("build_info");
const ess = @import("ess/ess.zig");
const SticksCalibration = @import("calibrate.zig").SticksCalibration;
const TriggersCalibration = @import("calibrate.zig").TriggersCalibration;
const ViGEmConfig = @import("bridge/bridge.zig").ViGEmBridge.Config;

const json_branch_quota = 2000;

pub const Driver = enum {
    vigem,

    pub fn jsonStringify(
        value: Driver,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        _ = options;
        try out_stream.writeByte('"');
        try out_stream.writeAll(std.meta.tagName(value));
        try out_stream.writeByte('"');
    }
};

pub const Config = struct {
    driver: Driver = .vigem,
    vigem_config: ViGEmConfig = .{ .pad = .ds4 },
    calibration: struct {
        enabled: bool = false,
        stick_data: ?SticksCalibration = null,
        trigger_data: ?TriggersCalibration = null,
    } = .{},
    ess: struct {
        inversion_mapping: ?ess.Mapping = null,
    } = .{},
    analog_scale: f32 = 1.0,
    rumble: RumbleSetting = .on,
    input_server: struct {
        enabled: bool = false,
        port: u16 = 4096,
    } = .{},

    pub const RumbleSetting = enum {
        on,
        off,
        emulator,

        pub fn jsonStringify(
            value: RumbleSetting,
            options: std.json.StringifyOptions,
            out_stream: anytype,
        ) @TypeOf(out_stream).Error!void {
            _ = options;
            try out_stream.writeByte('"');
            try out_stream.writeAll(std.meta.tagName(value));
            try out_stream.writeByte('"');
        }
    };
};

pub const ConfigFile = struct {
    @"$schema": []const u8,
    current_profile: []const u8,
    profiles: []Profile,

    pub const Profile = struct {
        name: []const u8,
        config: Config,
    };

    pub const path = build_info.config_path;

    pub fn init(allocator: std.mem.Allocator, default: Config) !ConfigFile {
        var profiles = try allocator.alloc(Profile, 1);
        profiles[0] = .{ .name = "default", .config = default };

        return ConfigFile{
            .@"$schema" = try std.mem.join(allocator, "/", &[_][]const u8{
                build_info.usercontent_url,
                build_info.schema_rel_path,
            }),
            .current_profile = "default",
            .profiles = profiles,
        };
    }

    pub fn load(allocator: std.mem.Allocator) !?ConfigFile {
        return try parseFromFile(ConfigFile, allocator, path);
    }

    pub fn save(self: ConfigFile, allocator: std.mem.Allocator) !void {
        try saveToFile(path, allocator, self);
    }

    pub fn lookupProfile(self: ConfigFile, name: []const u8) ?*Profile {
        for (self.profiles) |s, i| {
            if (std.mem.eql(u8, s.name, name)) {
                return &self.profiles[i];
            }
        }

        return null;
    }
};

fn parseFromFile(comptime T: type, allocator: std.mem.Allocator, path: []const u8) !?T {
    const file_contents: ?[]const u8 = blk: {
        const exe_dir_path = std.fs.selfExeDirPathAlloc(allocator) catch break :blk null;
        defer allocator.free(exe_dir_path);

        var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
        defer exe_dir.close();

        const file = exe_dir.openFile(path, .{}) catch break :blk null;
        defer file.close();

        break :blk try file.readToEndAlloc(allocator, std.math.maxInt(usize));
    };

    if (file_contents) |s| {
        defer allocator.free(s);
        var stream = std.json.TokenStream.init(s);

        @setEvalBranchQuota(json_branch_quota);
        return try std.json.parse(T, &stream, .{ .allocator = allocator });
    } else {
        return null;
    }
}

fn saveToFile(path: []const u8, allocator: std.mem.Allocator, config: anytype) !void {
    const exe_dir_path = try std.fs.selfExeDirPathAlloc(allocator);
    defer allocator.free(exe_dir_path);

    var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
    defer exe_dir.close();

    var file = try exe_dir.createFile(path, .{});
    defer file.close();

    @setEvalBranchQuota(json_branch_quota);
    try std.json.stringify(config, .{ .whitespace = .{} }, file.writer());
}

const CallbackError = error{};
const CallbackWriter = std.io.Writer(CallbackContext, CallbackError, CallbackContext.write);

const CallbackContext = struct {
    callback: Callback,
    userdata: ?*anyopaque,

    pub const Callback = fn (
        bytes_ptr: [*c]const u8,
        bytes_len: usize,
        userdata: ?*anyopaque,
    ) callconv(.C) usize;

    pub fn write(self: CallbackContext, bytes: []const u8) CallbackError!usize {
        return self.callback(bytes.ptr, bytes.len, self.userdata);
    }
};

export fn formatConfigJson(
    json_ptr: [*c]const u8,
    json_len: usize,
    callback: CallbackContext.Callback,
    userdata: ?*anyopaque,
) void {
    @setEvalBranchQuota(json_branch_quota);
    const unformatted = (json_ptr orelse return)[0..json_len];
    const allocator = std.heap.c_allocator;
    var writer = CallbackWriter{ .context = .{ .callback = callback, .userdata = userdata } };
    var buffered = std.io.bufferedWriter(writer);
    defer buffered.flush() catch |err| {
        std.debug.panic("Failed to flush buffered callback writer: {}", .{err});
    };

    var stream = std.json.TokenStream.init(unformatted);
    const options = std.json.ParseOptions{ .allocator = allocator };
    const config = std.json.parse(ConfigFile, &stream, options) catch |err| {
        std.debug.panic("Failed to parse json: {}", .{err});
    };
    defer std.json.parseFree(ConfigFile, config, options);

    std.json.stringify(config, .{ .whitespace = .{} }, buffered.writer()) catch |err| {
        std.debug.panic("Failed to stringify json: {}", .{err});
    };
}
