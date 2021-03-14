const std = @import("std");
const glad = @import("glad.zig");
const glfw = @import("glfw.zig");
const Input = @import("../adapter.zig").Input;
const Context = @import("root").Context;
const print = std.debug.print;

const a_button_color = [_]f32{ 0.0 / 255.0, 188.0 / 255.0, 142.0 / 255.0 };
const b_button_color = [_]f32{ 255.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0 };
const z_button_color = [_]f32{ 85.0 / 255.0, 0.0 / 255.0, 173.0 / 255.0 };
const c_stick_color = [_]f32{ 255.0 / 255.0, 228.0 / 255.0, 0.0 / 255.0 };

const window_x = 512;
const window_y = 512;

pub fn show(context: *const Context) !void {
    try glfw.init();
    defer glfw.terminate() catch unreachable;

    try glfw.windowHint(glfw.WindowHint.ContextVersionMajor, 3);
    try glfw.windowHint(glfw.WindowHint.ContextVersionMinor, 3);
    try glfw.windowHint(glfw.WindowHint.OpenGLProfile, @enumToInt(glfw.GLProfileAttribute.OpenglCoreProfile));

    const window = try glfw.createWindow(window_x, window_y, "Input Viewer", null, null);
    try glfw.makeContextCurrent(window);

    // wait for vsync to reduce cpu usage
    try glfw.swapInterval(1);

    var gl: glad.gl = .{};
    try gl.load(glfw.GLFWError, glfw.getProcAddress);
    try glfw.setWindowUserPointer(window, &gl);

    _ = try glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    const vertex_shader_source: [*:0]const u8 = @embedFile("vertex.glsl");
    const vertex_shader = gl.CreateShader(glad.GL_VERTEX_SHADER);
    gl.ShaderSource(vertex_shader, 1, &vertex_shader_source, null);
    gl.CompileShader(vertex_shader);

    const fragment_shader_source: [*:0]const u8 = @embedFile("sdf_button_fragment.glsl");
    const fragment_shader = gl.CreateShader(glad.GL_FRAGMENT_SHADER);
    gl.ShaderSource(fragment_shader, 1, &fragment_shader_source, null);
    gl.CompileShader(fragment_shader);

    const shader_program = gl.CreateProgram();
    gl.AttachShader(shader_program, vertex_shader);
    gl.AttachShader(shader_program, fragment_shader);
    gl.LinkProgram(shader_program);

    gl.DeleteShader(vertex_shader);
    gl.DeleteShader(fragment_shader);

    const bean_sdf: []const u8 = @embedFile("bean-sdf.gray");

    var bean_texture: u32 = undefined;
    gl.GenTextures(1, &bean_texture);
    gl.BindTexture(glad.GL_TEXTURE_2D, bean_texture);
    gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_WRAP_S, glad.GL_CLAMP_TO_BORDER);
    gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_WRAP_T, glad.GL_CLAMP_TO_BORDER);
    gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_MIN_FILTER, glad.GL_LINEAR);
    gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_MAG_FILTER, glad.GL_LINEAR);

    gl.TexImage2D(glad.GL_TEXTURE_2D, 0, glad.GL_RED, 64, 64, 0, glad.GL_RED, glad.GL_UNSIGNED_BYTE, bean_sdf.ptr);
    gl.GenerateMipmap(glad.GL_TEXTURE_2D);

    gl.ActiveTexture(glad.GL_TEXTURE0);
    gl.BindTexture(glad.GL_TEXTURE_2D, bean_texture);

    const vertices = [_]f32{
        // positions \ texture coords
        -0.5, 0.5,  0.0, 1.0,
        -0.5, -0.5, 0.0, 0.0,
        0.5,  -0.5, 1.0, 0.0,
        0.5,  0.5,  1.0, 1.0,
    };

    const indices = [_]c_uint{
        0, 1, 2,
        0, 3, 2,
    };

    const vertex_bytes: []const u8 = std.mem.sliceAsBytes(&vertices);
    const indices_bytes: []const u8 = std.mem.sliceAsBytes(&indices);

    var vbo: u32 = undefined;
    var vao: u32 = undefined;
    var ebo: u32 = undefined;
    gl.GenVertexArrays(1, &vao);
    gl.GenBuffers(1, &vbo);
    gl.GenBuffers(1, &ebo);

    gl.BindVertexArray(vao);

    gl.BindBuffer(glad.GL_ARRAY_BUFFER, vbo);
    gl.BufferData(glad.GL_ARRAY_BUFFER, vertex_bytes.len, vertex_bytes.ptr, glad.GL_STATIC_DRAW);

    gl.BindBuffer(glad.GL_ELEMENT_ARRAY_BUFFER, ebo);
    gl.BufferData(glad.GL_ELEMENT_ARRAY_BUFFER, indices_bytes.len, indices_bytes.ptr, glad.GL_STATIC_DRAW);

    // position attribute
    gl.VertexAttribPointer(0, 2, glad.GL_FLOAT, glad.GL_FALSE, 4 * @sizeOf(f32), @intToPtr(?*const c_void, 0));
    gl.EnableVertexAttribArray(0);

    // texture coords attribute
    gl.VertexAttribPointer(1, 2, glad.GL_FLOAT, glad.GL_FALSE, 4 * @sizeOf(f32), @intToPtr(?*const c_void, 2 * @sizeOf(f32)));
    gl.EnableVertexAttribArray(1);

    gl.BindBuffer(glad.GL_ARRAY_BUFFER, 0);
    gl.BindVertexArray(0);

    while (!try glfw.windowShouldClose(window)) {
        gl.ClearColor(0.0, 0.0, 0.0, 1.0);
        gl.Clear(glad.GL_COLOR_BUFFER_BIT);

        gl.UseProgram(shader_program);

        {
            const x: [*:0]const u8 = "center";
            gl.Uniform2f(gl.GetUniformLocation(shader_program, x), 0.0, 0.0);
        }

        {
            const x: [*:0]const u8 = "size";
            gl.Uniform1f(gl.GetUniformLocation(shader_program, x), 0.5);
        }

        {
            const x: [*:0]const u8 = "color";
            gl.Uniform3fv(gl.GetUniformLocation(shader_program, x), 1, &a_button_color);
        }

        {
            const is_pressed = blk: {
                if (context.last_input) |last| {
                    break :blk last.button_a;
                } else {
                    break :blk false;
                }
            };

            const x: [*:0]const u8 = "pressed";
            gl.Uniform1i(gl.GetUniformLocation(shader_program, x), @boolToInt(is_pressed));
        }

        gl.BindVertexArray(vao);
        gl.DrawElements(glad.GL_TRIANGLES, 6, glad.GL_UNSIGNED_INT, @intToPtr(?*const c_void, 0));

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    const gl = @ptrCast(*glad.gl, @alignCast(8, glfw.getWindowUserPointer(window) catch unreachable));
    print("window size changed: {} {}\n", .{ width, height });
    gl.Viewport(0, 0, @intCast(u32, width), @intCast(u32, height));
}
