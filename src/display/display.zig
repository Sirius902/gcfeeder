const std = @import("std");
const glad = @import("glad.zig");
const zgl = @import("zgl.zig");
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

    try glad.gl_context.load(glfw.GLFWError, glfw.getProcAddress);

    _ = try glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    const vertex_shader_source: []const u8 = @embedFile("vertex.glsl");
    const vertex_shader = zgl.Shader.create(zgl.ShaderType.vertex);
    vertex_shader.source(1, &vertex_shader_source);
    vertex_shader.compile();

    const fragment_shader_source: []const u8 = @embedFile("stick_fragment.glsl");
    const fragment_shader = zgl.Shader.create(zgl.ShaderType.fragment);
    fragment_shader.source(1, &fragment_shader_source);
    fragment_shader.compile();

    const shader_program = zgl.Program.create();
    shader_program.attach(vertex_shader);
    shader_program.attach(fragment_shader);
    shader_program.link();

    vertex_shader.delete();
    fragment_shader.delete();

    // const bean_sdf: []const u8 = @embedFile("bean-sdf.gray");

    // var bean_texture: u32 = undefined;
    // gl.GenTextures(1, &bean_texture);
    // gl.BindTexture(glad.GL_TEXTURE_2D, bean_texture);
    // gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_WRAP_S, glad.GL_CLAMP_TO_BORDER);
    // gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_WRAP_T, glad.GL_CLAMP_TO_BORDER);
    // gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_MIN_FILTER, glad.GL_LINEAR);
    // gl.TexParameteri(glad.GL_TEXTURE_2D, glad.GL_TEXTURE_MAG_FILTER, glad.GL_LINEAR);

    // gl.TexImage2D(glad.GL_TEXTURE_2D, 0, glad.GL_RED, 64, 64, 0, glad.GL_RED, glad.GL_UNSIGNED_BYTE, bean_sdf.ptr);
    // gl.GenerateMipmap(glad.GL_TEXTURE_2D);

    // gl.ActiveTexture(glad.GL_TEXTURE0);
    // gl.BindTexture(glad.GL_TEXTURE_2D, bean_texture);

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

    var vbo = zgl.Buffer.gen();
    var vao = zgl.VertexArray.gen();
    var ebo = zgl.Buffer.gen();

    vao.bind();

    vbo.bind(zgl.BufferTarget.array_buffer);
    vbo.data(u8, vertex_bytes, zgl.BufferUsage.static_draw);

    ebo.bind(zgl.BufferTarget.element_array_buffer);
    ebo.data(u8, indices_bytes, zgl.BufferUsage.static_draw);

    // position attribute
    zgl.vertexAttribPointer(0, 2, zgl.Type.float, false, 4 * @sizeOf(f32), 0);
    zgl.enableVertexAttribArray(0);

    // texture coords attribute
    zgl.vertexAttribPointer(1, 2, zgl.Type.float, false, 4 * @sizeOf(f32), 2 * @sizeOf(f32));
    zgl.enableVertexAttribArray(1);

    while (!try glfw.windowShouldClose(window)) {
        zgl.clearColor(0.0, 0.0, 0.0, 1.0);
        zgl.clear(.{ .color = true });

        shader_program.use();

        glad.gl_context.Uniform3fv(glad.gl_context.GetUniformLocation(@enumToInt(shader_program), "color"), 1, &a_button_color);

        {
            const xp: f32 = blk: {
                if (context.last_input) |last| {
                    break :blk @intToFloat(f32, last.stick_x) / 255.0;
                } else {
                    break :blk 0.0;
                }
            };

            const yp: f32 = blk: {
                if (context.last_input) |last| {
                    break :blk @intToFloat(f32, last.stick_y) / 255.0;
                } else {
                    break :blk 0.0;
                }
            };

            glad.gl_context.Uniform2f(glad.gl_context.GetUniformLocation(@enumToInt(shader_program), "pos"), 1.0 - xp, 1.0 - yp);
        }

        vao.bind();
        zgl.drawElements(zgl.PrimitiveType.triangles, 6, zgl.ElementType.u32, null);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    zgl.viewport(0, 0, @intCast(u32, width), @intCast(u32, height));
}
