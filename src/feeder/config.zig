const std = @import("std");
const Calibration = @import("calibrate.zig").Calibration;
const ViGEmConfig = @import("bridge/bridge.zig").ViGEmBridge.Config;

pub const Driver = enum {
    vjoy,
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

    pub const path = "config.json";

    pub fn load(allocator: std.mem.Allocator) !?Config {
        const payload: ?[]const u8 = blk: {
            const exe_dir_path = std.fs.selfExeDirPathAlloc(allocator) catch break :blk null;
            defer allocator.free(exe_dir_path);

            var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
            defer exe_dir.close();

            const file = exe_dir.openFile(path, .{}) catch break :blk null;
            defer file.close();

            break :blk try file.readToEndAlloc(allocator, std.math.maxInt(usize));
        };

        if (payload) |p| {
            defer allocator.free(p);
            var stream = std.json.TokenStream.init(p);
            return try std.json.parse(Config, &stream, .{});
        } else {
            return null;
        }
    }

    pub fn save(self: Config, allocator: std.mem.Allocator) !void {
        const exe_dir_path = try std.fs.selfExeDirPathAlloc(allocator);
        defer allocator.free(exe_dir_path);

        var exe_dir = try std.fs.cwd().openDir(exe_dir_path, .{});
        defer exe_dir.close();

        var file = try exe_dir.createFile(path, .{});
        defer file.close();

        const writer = file.writer();
        try std.json.stringify(self, .{ .whitespace = .{} }, writer);
    }
};
