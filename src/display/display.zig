const std = @import("std");
const glad = @import("glad");
const zgl = @import("zgl");
const zlm = @import("zlm");
const glfw = @import("zglfw");
const Input = @import("../adapter.zig").Input;
const Calibration = @import("../adapter.zig").Calibration;
const Context = @import("root").Context;

const Display = struct {
    const vertex_shader_source: []const u8 = @embedFile("vertex.glsl");
    const circle_button_shader_source: []const u8 = @embedFile("circle_button_fragment.glsl");
    const sdf_button_shader_source: []const u8 = @embedFile("sdf_button_fragment.glsl");
    const trigger_shader_source: []const u8 = @embedFile("trigger_fragment.glsl");
    const stick_shader_source: []const u8 = @embedFile("stick_fragment.glsl");

    const bean_sdf = @embedFile("bean-sdf.gray");
    const z_button_sdf = @embedFile("z-button-sdf.gray");

    const main_color = [_]f32{ 0.95, 0.95, 0.95 };
    const a_button_color = [_]f32{ 0.0 / 255.0, 188.0 / 255.0, 142.0 / 255.0 };
    const b_button_color = [_]f32{ 255.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0 };
    const z_button_color = [_]f32{ 85.0 / 255.0, 0.0 / 255.0, 173.0 / 255.0 };
    const c_stick_color = [_]f32{ 255.0 / 255.0, 228.0 / 255.0, 0.0 / 255.0 };

    const buttons_center = comptime zlm.Mat4.createTranslationXYZ(0.5, 0.0, 0.0);

    circle_button_program: zgl.Program,
    sdf_button_program: zgl.Program,
    trigger_program: zgl.Program,
    stick_program: zgl.Program,
    vbo: zgl.Buffer,
    vao: zgl.VertexArray,
    ebo: zgl.Buffer,

    pub fn init() Display {
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

        return Display{
            .circle_button_program = circle_button_program,
            .sdf_button_program = sdf_button_program,
            .trigger_program = trigger_program,
            .stick_program = stick_program,
            .vbo = vbo,
            .vao = vao,
            .ebo = ebo,
        };
    }

    pub fn draw(self: Display, context: *const Context) void {
        self.vao.bind();

        self.drawCircleButtons(context);
        self.drawSdfButtons(context);
        self.drawSticks(context);
        self.drawTriggers(context);
    }

    fn drawCircleButtons(self: Display, context: *const Context) void {
        const program = self.circle_button_program;
        program.use();
        // a button
        {
            const model = zlm.Mat4.createUniformScale(0.75).mul(buttons_center);

            // use programUniform1i instead because uniform1i has a name conflict
            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_a else false),
            );
            program.uniform3f(program.uniformLocation("color"), a_button_color[0], a_button_color[1], a_button_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // b button
        {
            const model = zlm.Mat4.createUniformScale(0.45).mul(
                buttons_center.mul(
                    zlm.Mat4.createTranslationXYZ(-0.25, -0.15, 0.0),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_b else false),
            );
            program.uniform3f(program.uniformLocation("color"), b_button_color[0], b_button_color[1], b_button_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // start button
        {
            const model = zlm.Mat4.createUniformScale(0.3).mul(
                buttons_center.mul(
                    zlm.Mat4.createTranslationXYZ(-0.325, 0.0475, 0.0),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_start else false),
            );
            program.uniform3f(program.uniformLocation("color"), main_color[0], main_color[1], main_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
    }

    fn drawSdfButtons(self: Display, context: *const Context) void {
        const bean_scale = zlm.Mat4.createUniformScale(0.325);

        const program = self.sdf_button_program;
        program.use();
        program.uniform3f(program.uniformLocation("color"), main_color[0], main_color[1], main_color[2]);
        zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 0);
        // y button
        {
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(110.0)).mul(
                bean_scale.mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(-0.1, 0.225, 0.0),
                    ),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_y else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // x button
        {
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(225.0)).mul(
                bean_scale.mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(0.25, 0.0, 0.0),
                    ),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_x else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // z button
        {
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(-10.0)).mul(
                zlm.Mat4.createUniformScale(0.25).mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(0.185, 0.285, 0.0),
                    ),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (context.last_input) |last| last.button_z else false),
            );
            zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 1);
            program.uniform3f(program.uniformLocation("color"), z_button_color[0], z_button_color[1], z_button_color[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
    }

    fn drawSticks(self: Display, context: *const Context) void {
        const scale = zlm.Mat4.createUniformScale(0.6);

        const program = self.stick_program;
        program.use();
        // main stick
        {
            const model = scale.mul(
                zlm.Mat4.createTranslationXYZ(-0.65, 0.0, 0.0),
            );

            const x = @floatCast(f32, if (context.last_input) |last|
                1.0 - Calibration.main_stick.normalize(last.stick_x)
            else
                0.5);

            const y = @floatCast(f32, if (context.last_input) |last|
                1.0 - Calibration.main_stick.normalize(last.stick_y)
            else
                0.5);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(false));
            program.uniform3f(program.uniformLocation("color"), main_color[0], main_color[1], main_color[2]);

            glad.gl_context.Uniform2fv(
                glad.gl_context.GetUniformLocation(@enumToInt(program), "pos"),
                1,
                &[_]f32{ x, y },
            );

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // c stick
        {
            const model = scale.mul(
                zlm.Mat4.createTranslationXYZ(-0.15, 0.0, 0.0),
            );

            const x = @floatCast(f32, if (context.last_input) |last|
                1.0 - Calibration.c_stick.normalize(last.substick_x)
            else
                0.5);

            const y = @floatCast(f32, if (context.last_input) |last|
                1.0 - Calibration.c_stick.normalize(last.substick_y)
            else
                0.5);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(true));
            program.uniform3f(program.uniformLocation("color"), c_stick_color[0], c_stick_color[1], c_stick_color[2]);

            glad.gl_context.Uniform2fv(
                glad.gl_context.GetUniformLocation(@enumToInt(program), "pos"),
                1,
                &[_]f32{ x, y },
            );

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
    }

    fn drawTriggers(self: Display, context: *const Context) void {
        const scale = zlm.Mat4.createUniformScale(0.375);

        const program = self.trigger_program;
        program.use();
        program.uniform3f(program.uniformLocation("color"), main_color[0], main_color[1], main_color[2]);
        // left trigger
        {
            const model = scale.mul(
                zlm.Mat4.createTranslationXYZ(-0.65, 0.35, 0.0),
            );

            const fill = @floatCast(f32, if (context.last_input) |last|
                Calibration.trigger_range.normalize(last.trigger_left)
            else
                0.0);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(false));
            program.uniform1f(program.uniformLocation("fill"), fill);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, null);
        }
        // right trigger
        {
            const model = scale.mul(
                zlm.Mat4.createTranslationXYZ(-0.15, 0.35, 0.0),
            );

            const fill = @floatCast(f32, if (context.last_input) |last|
                Calibration.trigger_range.normalize(last.trigger_right)
            else
                0.0);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(true));
            program.uniform1f(program.uniformLocation("fill"), fill);
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

        const z_button_texture = zgl.Texture.create(.@"2d");
        z_button_texture.bindTo(1);
        z_button_texture.parameter(.wrap_s, .clamp_to_border);
        z_button_texture.parameter(.wrap_t, .clamp_to_border);
        z_button_texture.parameter(.min_filter, .linear);
        z_button_texture.parameter(.mag_filter, .linear);
        z_button_texture.storage2D(1, .r8, 64, 64);
        z_button_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, z_button_sdf);
    }
};

pub fn show(context: *const Context) !void {
    try glfw.init();
    defer glfw.terminate() catch unreachable;

    try glfw.windowHint(.ContextVersionMajor, 3);
    try glfw.windowHint(.ContextVersionMinor, 3);
    try glfw.windowHint(.OpenGLProfile, @enumToInt(glfw.GLProfileAttribute.OpenglCoreProfile));

    const window = try glfw.createWindow(512, 512, "Input Viewer", null, null);
    try glfw.makeContextCurrent(window);

    // wait for vsync to reduce cpu usage
    try glfw.swapInterval(1);
    _ = try glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    try glad.gl_context.load(glfw.GLFWError, glfw.getProcAddress);

    const display = Display.init();

    while (!try glfw.windowShouldClose(window)) {
        zgl.clearColor(0.0, 0.0, 0.0, 1.0);
        zgl.clear(.{ .color = true });

        display.draw(context);

        try glfw.swapBuffers(window);
        try glfw.pollEvents();
    }
}

fn framebufferSizeCallback(window: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    zgl.viewport(0, 0, @intCast(u32, width), @intCast(u32, height));
}
