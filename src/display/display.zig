const std = @import("std");
const glad = @import("../zgl/glad.zig");
const zgl = @import("../zgl/zgl.zig");
const zlm = @import("../zlm/zlm.zig");
const glfw = @import("../zglfw/src/glfw.zig");
const Input = @import("../adapter.zig").Input;
const Context = @import("root").Context;
const print = std.debug.print;

const window_x = 512;
const window_y = 512;

const GraphicsContext = struct {
    const vertex_shader_source: []const u8 = @embedFile("vertex.glsl");
    const circle_button_shader_source: []const u8 = @embedFile("circle_button_fragment.glsl");
    const sdf_button_shader_source: []const u8 = @embedFile("sdf_button_fragment.glsl");
    const trigger_shader_source: []const u8 = @embedFile("trigger_fragment.glsl");
    const stick_shader_source: []const u8 = @embedFile("stick_fragment.glsl");

    const bean_sdf = @embedFile("bean-sdf.gray");

    const a_button_color = [_]f32{ 0.0 / 255.0, 188.0 / 255.0, 142.0 / 255.0 };
    const b_button_color = [_]f32{ 255.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0 };
    const z_button_color = [_]f32{ 85.0 / 255.0, 0.0 / 255.0, 173.0 / 255.0 };
    const c_stick_color = [_]f32{ 255.0 / 255.0, 228.0 / 255.0, 0.0 / 255.0 };

    circle_button_program: zgl.Program,
    sdf_button_program: zgl.Program,
    trigger_program: zgl.Program,
    stick_program: zgl.Program,
    vbo: zgl.Buffer,
    vao: zgl.VertexArray,
    ebo: zgl.Buffer,

    pub fn init() GraphicsContext {
        const vertex_shader = zgl.Shader.create(.vertex);
        defer vertex_shader.delete();
        vertex_shader.source(1, &vertex_shader_source);
        vertex_shader.compile();

        const circle_button_shader = zgl.Shader.create(.fragment);
        defer circle_button_shader.delete();
        circle_button_shader.source(1, &circle_button_shader_source);
        circle_button_shader.compile();

        const sdf_button_shader = zgl.Shader.create(.fragment);
        defer sdf_button_shader.delete();
        sdf_button_shader.source(1, &sdf_button_shader_source);
        sdf_button_shader.compile();

        const trigger_shader = zgl.Shader.create(.fragment);
        defer trigger_shader.delete();
        trigger_shader.source(1, &trigger_shader_source);
        trigger_shader.compile();

        const stick_shader = zgl.Shader.create(.fragment);
        defer stick_shader.delete();
        stick_shader.source(1, &stick_shader_source);
        stick_shader.compile();

        const circle_button_program = zgl.Program.create();
        circle_button_program.attach(vertex_shader);
        circle_button_program.attach(circle_button_shader);
        circle_button_program.link();

        const sdf_button_program = zgl.Program.create();
        sdf_button_program.attach(vertex_shader);
        sdf_button_program.attach(sdf_button_shader);
        sdf_button_program.link();

        const trigger_program = zgl.Program.create();
        trigger_program.attach(vertex_shader);
        trigger_program.attach(trigger_shader);
        trigger_program.link();

        const stick_program = zgl.Program.create();
        stick_program.attach(vertex_shader);
        stick_program.attach(stick_shader);
        stick_program.link();

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

        const vbo = zgl.Buffer.gen();
        const vao = zgl.VertexArray.gen();
        const ebo = zgl.Buffer.gen();

        vao.bind();

        vbo.bind(.array_buffer);
        vbo.data(f32, &vertices, .static_draw);

        ebo.bind(.element_array_buffer);
        ebo.data(u32, &indices, .static_draw);

        // position attribute
        zgl.vertexAttribPointer(0, 2, .float, false, 4 * @sizeOf(f32), 0);
        zgl.enableVertexAttribArray(0);

        // texture coords attribute
        zgl.vertexAttribPointer(1, 2, .float, false, 4 * @sizeOf(f32), 2 * @sizeOf(f32));
        zgl.enableVertexAttribArray(1);

        loadTextures();

        return GraphicsContext{
            .circle_button_program = circle_button_program,
            .sdf_button_program = sdf_button_program,
            .trigger_program = trigger_program,
            .stick_program = stick_program,
            .vbo = vbo,
            .vao = vao,
            .ebo = ebo,
        };
    }

    pub fn draw(self: GraphicsContext, context: *const Context) void {
        const buttons_center = comptime zlm.Mat4.createTranslationXYZ(0.5, 0.0, 0.0);

        self.vao.bind();

        var program = self.circle_button_program;
        program.use();
        // a button
        {
            // use programUniform1i instead because uniform1i has a name conflict
            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_a else false),
            );
            program.uniform3f(program.uniformLocation("color"), a_button_color[0], a_button_color[1], a_button_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{buttons_center.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // b button
        {
            const model = comptime buttons_center.mul(zlm.Mat4.createTranslationXYZ(-0.3, -0.3, 0.0));

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_b else false),
            );
            program.uniform3f(program.uniformLocation("color"), b_button_color[0], b_button_color[1], b_button_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
    }

    fn loadTextures() void {
        const bean_texture = zgl.Texture.create(.@"2d");
        bean_texture.bindTo(0);
        bean_texture.parameter(.wrap_s, .clamp_to_border);
        bean_texture.parameter(.wrap_t, .clamp_to_border);
        bean_texture.parameter(.min_filter, .linear);
        bean_texture.parameter(.mag_filter, .linear);
        bean_texture.storage2D(1, .r8, 64, 64);
        bean_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, bean_sdf);
    }
};

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

    const gfx = GraphicsContext.init();

    while (!try glfw.windowShouldClose(window)) {
        zgl.clearColor(0.0, 0.0, 0.0, 1.0);
        zgl.clear(.{ .color = true });

        gfx.draw(context);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    zgl.viewport(0, 0, @intCast(u32, width), @intCast(u32, height));
}
