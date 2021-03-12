const std = @import("std");
const glad = @import("glad.zig");
const glfw = @import("glfw.zig");
const Input = @import("../adapter.zig").Input;
const print = std.debug.print;

const window_x = 800;
const window_y = 600;

pub fn show() !void {
    try glfw.init();
    defer glfw.terminate() catch unreachable;

    try glfw.windowHint(glfw.WindowHint.ContextVersionMajor, 3);
    try glfw.windowHint(glfw.WindowHint.ContextVersionMinor, 3);
    try glfw.windowHint(glfw.WindowHint.OpenGLProfile, @enumToInt(glfw.GLProfileAttribute.OpenglCoreProfile));

    const window = try glfw.createWindow(window_x, window_y, "Input Viewer", null, null);
    try glfw.makeContextCurrent(window);

    var gl: glad.gl = .{};
    try gl.load(glfw.GLFWError, glfw.getProcAddress);
    try glfw.setWindowUserPointer(window, &gl);

    _ = try glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    const vertex_shader_source: [*:0]const u8 = @embedFile("vertex.glsl");
    const vertex_shader = gl.CreateShader.?(glad.GL_VERTEX_SHADER);
    gl.ShaderSource.?(vertex_shader, 1, &vertex_shader_source, null);
    gl.CompileShader.?(vertex_shader);

    const fragment_shader_source: [*:0]const u8 = @embedFile("fragment.glsl");
    const fragment_shader = gl.CreateShader.?(glad.GL_FRAGMENT_SHADER);
    gl.ShaderSource.?(fragment_shader, 1, &fragment_shader_source, null);
    gl.CompileShader.?(fragment_shader);

    const shader_program = gl.CreateProgram.?();
    gl.AttachShader.?(shader_program, vertex_shader);
    gl.AttachShader.?(shader_program, fragment_shader);
    gl.LinkProgram.?(shader_program);

    gl.DeleteShader.?(vertex_shader);
    gl.DeleteShader.?(fragment_shader);

    const vertices = [_]f32{
        -0.5, -0.5, 0.0,
        0.5,  -0.5, 0.0,
        0.0,  0.5,  0.0,
    };

    const vertex_bytes: []const u8 = std.mem.sliceAsBytes(&vertices);
    gl.BufferData.?(glad.GL_ARRAY_BUFFER, vertex_bytes.len, vertex_bytes.ptr, glad.GL_STATIC_DRAW);

    var vbo: u32 = undefined;
    var vao: u32 = undefined;
    gl.GenVertexArrays.?(1, &vao);
    gl.GenBuffers.?(1, &vbo);
    gl.BindVertexArray.?(vao);

    gl.BindBuffer.?(glad.GL_ARRAY_BUFFER, vbo);
    gl.BufferData.?(glad.GL_ARRAY_BUFFER, vertex_bytes.len, vertex_bytes.ptr, glad.GL_STATIC_DRAW);

    gl.VertexAttribPointer.?(0, 3, glad.GL_FLOAT, glad.GL_FALSE, 3 * @sizeOf(f32), null);
    gl.EnableVertexAttribArray.?(0);

    gl.BindBuffer.?(glad.GL_ARRAY_BUFFER, 0);
    gl.BindVertexArray.?(0);

    while (!try glfw.windowShouldClose(window)) {
        gl.ClearColor.?(0.2, 0.3, 0.3, 1.0);
        gl.Clear.?(glad.GL_COLOR_BUFFER_BIT);

        gl.UseProgram.?(shader_program);
        gl.BindVertexArray.?(vao);
        gl.DrawArrays.?(glad.GL_TRIANGLES, 0, 3);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    const gl = @ptrCast(*glad.gl, @alignCast(8, glfw.getWindowUserPointer(window) catch unreachable));
    print("window size changed: {} {}\n", .{ width, height });
    gl.Viewport.?(0, 0, window_x, window_y);
}
