const std = @import("std");
const c = @cImport({
    @cInclude("gui.h");
});

const roboto_ttf: []u8 = blk: {
    const ttf_const = @embedFile("font/Roboto-Medium.ttf");
    var buf: [ttf_const.len]u8 = undefined;
    @memcpy(&buf, ttf_const, buf.len);
    break :blk &buf;
};

pub fn runImGui(allocator: std.mem.Allocator) !void {
    const ini_path = blk: {
        const exe_dir_path = try std.fs.selfExeDirPathAlloc(allocator);
        defer allocator.free(exe_dir_path);

        break :blk try std.mem.joinZ(allocator, "/", &[_][]const u8{ exe_dir_path, "imgui-gcfeeder.ini" });
    };
    defer allocator.free(ini_path);

    // Setup window
    _ = c.glfwSetErrorCallback(glfwErrorCallback);
    if (c.glfwInit() == 0)
        return error.GlfwInitFailed;

    // Decide GL+GLSL versions
    const glsl_version = "#version 130";
    // GL 3.0 + GLSL 130
    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MAJOR, 3);
    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MINOR, 0);
    //c.glfwWindowHint(c.GLFW_OPENGL_PROFILE, c.GLFW_OPENGL_CORE_PROFILE);  // 3.2+ only
    //c.glfwWindowHint(c.GLFW_OPENGL_FORWARD_COMPAT, gl.GL_TRUE);            // 3.0+ only

    // Create window with graphics context
    const window = c.glfwCreateWindow(1280, 720, "gcfeeder", null, null) orelse
        return error.GlfwCreateWindowFailed;
    c.glfwMakeContextCurrent(window);
    c.glfwSwapInterval(1); // Enable vsync

    var context = c.UIContext{
        .ttf_ptr = roboto_ttf.ptr,
        .ttf_len = roboto_ttf.len,
        .ini_path = ini_path.ptr,
        .window = window,
        .glsl_version = glsl_version,
    };
    _ = c.runImGui(&context);

    c.glfwDestroyWindow(window);
    c.glfwTerminate();
}

fn glfwErrorCallback(err: c_int, description: ?[*:0]const u8) callconv(.C) void {
    std.debug.print("Glfw Error {}: {s}\n", .{ err, description });
}