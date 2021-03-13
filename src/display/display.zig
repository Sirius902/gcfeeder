const std = @import("std");
const glad = @import("glad.zig");
const glfw = @import("glfw.zig");
const Input = @import("../adapter.zig").Input;
const print = std.debug.print;

const a_button_color = [_]f32{ 0.0 / 255.0, 188.0 / 255.0, 142.0 / 255.0 };
const b_button_color = [_]f32{ 255.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0 };
const z_button_color = [_]f32{ 85.0 / 255.0, 0.0 / 255.0, 173.0 / 255.0 };
const c_stick_color = [_]f32{ 255.0 / 255.0, 228.0 / 255.0, 0.0 / 255.0 };

const window_x = 480;
const window_y = 300;

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
        // positions
        -0.5, 0.5,
        -0.5, -0.5,
        0.5,  -0.5,
        -0.5, 0.5,
        0.5,  0.5,
        0.5,  -0.5,
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

    // position attribute
    gl.VertexAttribPointer.?(0, 2, glad.GL_FLOAT, glad.GL_FALSE, 2 * @sizeOf(f32), @intToPtr(?*const c_void, 0));
    gl.EnableVertexAttribArray.?(0);

    gl.BindBuffer.?(glad.GL_ARRAY_BUFFER, 0);
    gl.BindVertexArray.?(0);

    while (!try glfw.windowShouldClose(window)) {
        gl.ClearColor.?(0.0, 0.0, 0.0, 1.0);
        gl.Clear.?(glad.GL_COLOR_BUFFER_BIT);

        gl.UseProgram.?(shader_program);
        const x: [*:0]const u8 = "object_color";
        gl.Uniform3fv.?(gl.GetUniformLocation.?(shader_program, x), 1, &z_button_color);
        gl.BindVertexArray.?(vao);
        gl.DrawArrays.?(glad.GL_TRIANGLES, 0, 6);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    const gl = @ptrCast(*glad.gl, @alignCast(8, glfw.getWindowUserPointer(window) catch unreachable));
    print("window size changed: {} {}\n", .{ width, height });
    gl.Viewport.?(0, 0, @intCast(u32, width), @intCast(u32, height));
}
