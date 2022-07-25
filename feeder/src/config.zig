const std = @import("std");
const build_info = @import("build_info");
const Calibration = @import("calibrate.zig").Calibration;
const ViGEmConfig = @import("bridge/bridge.zig").ViGEmBridge.Config;

pub const Driver = enum {
    vigem,

    pub fn jsonStringify(
        value: Driver,
        options: std.json.StringifyOptions,
        out_stream: anytype,
    ) @TypeOf(out_stream).Error!void {
        _ = options;
        try out_stream.writeByte('"');
        try out_stream.writeAll(std.meta.fieldNames(Driver)[@enumToInt(value)]);
        try out_stream.writeByte('"');
    }
};

pub const Config = struct {
    driver: Driver = .vigem,
    vigem_config: ViGEmConfig = .{ .pad = .ds4 },
    calibration: ?Calibration = null,
};

pub const ConfigFile = struct {
    @"$schema": []const u8,
    default_set: []const u8,
    config_sets: []ConfigSet,

    pub const ConfigSet = struct {
        name: []const u8,
        config: Config,
    };

    pub const path = "gcfeeder.json";

    pub fn init(allocator: std.mem.Allocator, default: Config) !ConfigFile {
        var config_sets = try allocator.alloc(ConfigSet, 1);
        config_sets[0] = .{ .name = "default", .config = default };

        return ConfigFile{
            .@"$schema" = try std.mem.join(allocator, "/", &[_][]const u8{
                build_info.usercontent_url,
                "feeder/schema/gcfeeder.schema.json",
            }),
            .default_set = "default",
            .config_sets = config_sets,
        };
    }

    pub fn load(allocator: std.mem.Allocator) !?ConfigFile {
        return try parseFromFile(ConfigFile, allocator, path);
    }

    pub fn save(self: ConfigFile, allocator: std.mem.Allocator) !void {
        try saveToFile(path, allocator, self);
    }

    pub fn lookupConfigSet(self: *ConfigFile, name: []const u8) ?*Config {
        for (self.config_sets) |*s| {
            if (std.mem.eql(u8, s.name, name)) {
                return &s.config;
            }
        }

        return null;
    }

    pub fn migrateOldConfig(allocator: std.mem.Allocator) !void {
        const exe_dir_path = try std.fs.selfExeDirPathAlloc(allocator);
        defer allocator.free(exe_dir_path);

        var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
        defer exe_dir.close();

        exe_dir.access(path, .{}) catch {
            const old_path = "config.json";
            const old_config = (try parseFromFile(Config, allocator, old_path)) orelse return;
            try saveToFile(path, allocator, try init(allocator, old_config));
            try exe_dir.deleteFile(old_path);
        };
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

    try std.json.stringify(config, .{ .whitespace = .{} }, file.writer());
}
