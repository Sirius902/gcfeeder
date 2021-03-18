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

    try glfw.windowHint(.ContextVersionMajor, 3);
    try glfw.windowHint(.ContextVersionMinor, 3);
    try glfw.windowHint(.OpenGLProfile, @enumToInt(glfw.GLProfileAttribute.OpenglCoreProfile));

    const window = try glfw.createWindow(window_x, window_y, "Input Viewer", null, null);
    try glfw.makeContextCurrent(window);

    // wait for vsync to reduce cpu usage
    try glfw.swapInterval(1);
    _ = try glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    try glad.gl_context.load(glfw.GLFWError, glfw.getProcAddress);

    const vertex_shader_source: []const u8 = @embedFile("vertex.glsl");
    const vertex_shader = zgl.Shader.create(.vertex);
    vertex_shader.source(1, &vertex_shader_source);
    vertex_shader.compile();

    const fragment_shader_source: []const u8 = @embedFile("sdf_button_fragment.glsl");
    const fragment_shader = zgl.Shader.create(.fragment);
    fragment_shader.source(1, &fragment_shader_source);
    fragment_shader.compile();

    const shader_program = zgl.Program.create();
    shader_program.attach(vertex_shader);
    shader_program.attach(fragment_shader);
    shader_program.link();

    vertex_shader.delete();
    fragment_shader.delete();

    const bean_sdf: []const u8 = @embedFile("bean-sdf.gray");
    const bean_texture = zgl.Texture.create(.@"2d");
    bean_texture.bindTo(0);
    bean_texture.parameter(.wrap_s, .clamp_to_border);
    bean_texture.parameter(.wrap_t, .clamp_to_border);
    bean_texture.parameter(.min_filter, .linear);
    bean_texture.parameter(.mag_filter, .linear);
    bean_texture.storage2D(1, .r8, 64, 64);
    bean_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, bean_sdf.ptr);

    const vertices = [_]f32{
        // positions \ texture coords
        -0.5, 0.5,  0.0, 1.0,
        -0.5, -0.5, 0.0, 0.0,
        0.5,  -0.5, 1.0, 0.0,
        0.5,  0.5,  1.0, 1.0,
    };

    const indices = [_]u32{
        0, 1, 2,
        0, 3, 2,
    };

    const vertex_bytes: []const u8 = std.mem.sliceAsBytes(&vertices);
    const indices_bytes: []const u8 = std.mem.sliceAsBytes(&indices);

    const vbo = zgl.Buffer.gen();
    const vao = zgl.VertexArray.gen();
    const ebo = zgl.Buffer.gen();

    vao.bind();

    vbo.bind(.array_buffer);
    vbo.data(u8, vertex_bytes, .static_draw);

    ebo.bind(.element_array_buffer);
    ebo.data(u8, indices_bytes, .static_draw);

    // position attribute
    zgl.vertexAttribPointer(0, 2, .float, false, 4 * @sizeOf(f32), 0);
    zgl.enableVertexAttribArray(0);

    // texture coords attribute
    zgl.vertexAttribPointer(1, 2, .float, false, 4 * @sizeOf(f32), 2 * @sizeOf(f32));
    zgl.enableVertexAttribArray(1);

    while (!try glfw.windowShouldClose(window)) {
        zgl.clearColor(0.0, 0.0, 0.0, 1.0);
        zgl.clear(.{ .color = true });

        shader_program.use();

        glad.gl_context.Uniform3fv(
            glad.gl_context.GetUniformLocation(@enumToInt(shader_program), "color"),
            1,
            &a_button_color,
        );

        {
            const pressed = blk: {
                if (context.last_input) |last| {
                    break :blk last.button_a;
                } else {
                    break :blk false;
                }
            };

            glad.gl_context.Uniform1i(
                glad.gl_context.GetUniformLocation(@enumToInt(shader_program), "pressed"),
                @boolToInt(pressed),
            );
        }

        // {
        //     const xp: f32 = blk: {
        //         if (context.last_input) |last| {
        //             break :blk @intToFloat(f32, last.stick_x) / 255.0;
        //         } else {
        //             break :blk 0.0;
        //         }
        //     };

        //     const yp: f32 = blk: {
        //         if (context.last_input) |last| {
        //             break :blk @intToFloat(f32, last.stick_y) / 255.0;
        //         } else {
        //             break :blk 0.0;
        //         }
        //     };

        //     glad.gl_context.Uniform2f(
        //         glad.gl_context.GetUniformLocation(@enumToInt(shader_program), "pos"),
        //         1.0 - xp,
        //         1.0 - yp,
        //     );
        // }

        vao.bind();
        zgl.drawElements(.triangles, 6, .u32, null);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    zgl.viewport(0, 0, @intCast(u32, width), @intCast(u32, height));
}
