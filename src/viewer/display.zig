const std = @import("std");
const zgl = @import("zgl");
const zlm = @import("zlm");
const glfw = @import("zglfw");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;
const Context = @import("root").Context;

var window_width: u32 = 512;
var window_height: u32 = 512;

pub const Theme = struct {
    colors: struct {
        background: [3]f32,
        a_button: [3]f32,
        b_button: [3]f32,
        x_button: [3]f32,
        y_button: [3]f32,
        z_button: [3]f32,
        start_button: [3]f32,
        main_stick: [3]f32,
        c_stick: [3]f32,
        left_trigger: [3]f32,
        right_trigger: [3]f32,
    },

    pub const default = blk: {
        const main = [_]f32{ 0.95, 0.95, 0.95 };

        break :blk Theme{
            .colors = .{
                .background = [_]f32{ 0.0, 0.0, 0.0 },
                .a_button = [_]f32{ 0.0 / 255.0, 188.0 / 255.0, 142.0 / 255.0 },
                .b_button = [_]f32{ 255.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0 },
                .x_button = main,
                .y_button = main,
                .z_button = [_]f32{ 85.0 / 255.0, 0.0 / 255.0, 173.0 / 255.0 },
                .start_button = main,
                .main_stick = main,
                .c_stick = [_]f32{ 255.0 / 255.0, 228.0 / 255.0, 0.0 / 255.0 },
                .left_trigger = main,
                .right_trigger = main,
            },
        };
    };
};

const Display = struct {
    const vertex_shader_source: []const u8 = @embedFile("shader/vertex.glsl");
    const circle_button_shader_source: []const u8 = @embedFile("shader/circle_button_fragment.glsl");
    const sdf_button_shader_source: []const u8 = @embedFile("shader/sdf_button_fragment.glsl");
    const trigger_shader_source: []const u8 = @embedFile("shader/trigger_fragment.glsl");
    const stick_shader_source: []const u8 = @embedFile("shader/stick_fragment.glsl");

    const bean_sdf = @embedFile("sdf/bean-sdf.gray");
    const z_button_sdf = @embedFile("sdf/z-button-sdf.gray");
    const octagon_sdf = @embedFile("sdf/octagon-sdf.gray");

    const buttons_center = zlm.Mat4.createTranslationXYZ(0.5, 0.0, 0.0);

    circle_button_program: zgl.Program,
    sdf_button_program: zgl.Program,
    trigger_program: zgl.Program,
    stick_program: zgl.Program,
    vbo: zgl.Buffer,
    vao: zgl.VertexArray,
    ebo: zgl.Buffer,
    theme: Theme,

    pub fn init(theme: Theme) Display {
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
            .theme = theme,
        };
    }

    pub fn draw(self: Display, input: ?Input) void {
        self.vao.bind();

        const aspect = @intToFloat(f32, window_width) / @intToFloat(f32, window_height);
        const projection = zlm.Mat4.createOrthogonal(-aspect, aspect, -1.0, 1.0, -1.0, 1.0);

        const programs = [_]zgl.Program{
            self.circle_button_program,
            self.sdf_button_program,
            self.trigger_program,
            self.stick_program,
        };

        for (programs) |program| {
            program.use();

            program.uniformMatrix4(
                program.uniformLocation("projection"),
                false,
                &[_][4][4]f32{projection.fields},
            );
        }

        self.drawCircleButtons(input);
        self.drawSdfButtons(input);
        self.drawSticks(input);
        self.drawTriggers(input);
    }

    fn drawCircleButtons(self: Display, input: ?Input) void {
        const colors = &self.theme.colors;
        const program = self.circle_button_program;
        program.use();
        // a button
        {
            const scale = 1.5;
            const model = zlm.Mat4.createUniformScale(scale).mul(buttons_center);
            program.uniform1f(program.uniformLocation("scale"), scale);

            // use programUniform1i instead because uniform1i has a name conflict
            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_a else false),
            );
            program.uniform3f(program.uniformLocation("color"), colors.a_button[0], colors.a_button[1], colors.a_button[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // b button
        {
            const scale = 0.85;
            const model = zlm.Mat4.createUniformScale(scale).mul(
                buttons_center.mul(
                    zlm.Mat4.createTranslationXYZ(-0.225, -0.15, 0.0),
                ),
            );
            program.uniform1f(program.uniformLocation("scale"), scale);

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_b else false),
            );
            program.uniform3f(program.uniformLocation("color"), colors.b_button[0], colors.b_button[1], colors.b_button[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // start button
        {
            const scale = 0.625;
            const model = zlm.Mat4.createUniformScale(scale).mul(
                buttons_center.mul(
                    zlm.Mat4.createTranslationXYZ(-0.325, 0.0475, 0.0),
                ),
            );
            program.uniform1f(program.uniformLocation("scale"), scale);

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_start else false),
            );
            program.uniform3f(
                program.uniformLocation("color"),
                colors.start_button[0],
                colors.start_button[1],
                colors.start_button[2],
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawSdfButtons(self: Display, input: ?Input) void {
        const colors = &self.theme.colors;
        const bean_scale = 0.275;
        const bean_scale_mat = zlm.Mat4.createUniformScale(bean_scale);

        const program = self.sdf_button_program;
        program.use();
        zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 0);
        program.uniform1f(program.uniformLocation("scale"), bean_scale);
        // y button
        {
            program.uniform3f(
                program.uniformLocation("color"),
                colors.y_button[0],
                colors.y_button[1],
                colors.y_button[2],
            );
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(110.0)).mul(
                bean_scale_mat.mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(-0.1, 0.225, 0.0),
                    ),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_y else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // x button
        {
            program.uniform3f(
                program.uniformLocation("color"),
                colors.x_button[0],
                colors.x_button[1],
                colors.x_button[2],
            );
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(225.0)).mul(
                bean_scale_mat.mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(0.25, 0.0, 0.0),
                    ),
                ),
            );

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_x else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // z button
        {
            const scale = 0.225;
            const model = zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(-10.0)).mul(
                zlm.Mat4.createUniformScale(scale).mul(
                    buttons_center.mul(
                        zlm.Mat4.createTranslationXYZ(0.185, 0.285, 0.0),
                    ),
                ),
            );
            program.uniform1f(program.uniformLocation("scale"), scale);

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_z else false),
            );
            zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 1);
            program.uniform3f(program.uniformLocation("color"), colors.z_button[0], colors.z_button[1], colors.z_button[2]);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawSticks(self: Display, input: ?Input) void {
        const colors = &self.theme.colors;
        const scale = 0.6;
        const scale_mat = zlm.Mat4.createUniformScale(scale);

        const program = self.stick_program;
        program.use();
        zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 2);
        program.uniform1f(program.uniformLocation("scale"), scale);
        // main stick
        {
            const model = scale_mat.mul(
                zlm.Mat4.createTranslationXYZ(-0.65, 0.0, 0.0),
            );

            const x = @floatCast(f32, if (input) |in|
                1.0 - Calibration.main_stick.normalize(in.stick_x)
            else
                0.5);

            const y = @floatCast(f32, if (input) |in|
                1.0 - Calibration.main_stick.normalize(in.stick_y)
            else
                0.5);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(false));
            program.uniform3f(
                program.uniformLocation("color"),
                colors.main_stick[0],
                colors.main_stick[1],
                colors.main_stick[2],
            );

            zgl.uniform2f(program.uniformLocation("pos"), x, y);

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // c stick
        {
            const model = scale_mat.mul(
                zlm.Mat4.createTranslationXYZ(-0.15, 0.0, 0.0),
            );

            const x = @floatCast(f32, if (input) |in|
                1.0 - Calibration.c_stick.normalize(in.substick_x)
            else
                0.5);

            const y = @floatCast(f32, if (input) |in|
                1.0 - Calibration.c_stick.normalize(in.substick_y)
            else
                0.5);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(true));
            program.uniform3f(program.uniformLocation("color"), colors.c_stick[0], colors.c_stick[1], colors.c_stick[2]);

            zgl.uniform2f(program.uniformLocation("pos"), x, y);

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawTriggers(self: Display, input: ?Input) void {
        const colors = &self.theme.colors;
        const scale = 0.375;
        const scale_mat = zlm.Mat4.createUniformScale(scale);

        const program = self.trigger_program;
        program.use();
        program.uniform1f(program.uniformLocation("scale"), scale);
        // left trigger
        {
            program.uniform3f(
                program.uniformLocation("color"),
                colors.left_trigger[0],
                colors.left_trigger[1],
                colors.left_trigger[2],
            );
            const model = scale_mat.mul(
                zlm.Mat4.createTranslationXYZ(-0.65, 0.35, 0.0),
            );

            const fill = @floatCast(f32, if (input) |in|
                if (in.button_l) 1.0 else Calibration.trigger_range.normalize(in.trigger_left)
            else
                0.0);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(false));
            program.uniform1f(program.uniformLocation("fill"), fill);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // right trigger
        {
            program.uniform3f(
                program.uniformLocation("color"),
                colors.right_trigger[0],
                colors.right_trigger[1],
                colors.right_trigger[2],
            );
            const model = scale_mat.mul(
                zlm.Mat4.createTranslationXYZ(-0.15, 0.35, 0.0),
            );

            const fill = @floatCast(f32, if (input) |in|
                if (in.button_r) 1.0 else Calibration.trigger_range.normalize(in.trigger_right)
            else
                0.0);

            zgl.programUniform1i(program, program.uniformLocation("is_c_stick"), @boolToInt(true));
            program.uniform1f(program.uniformLocation("fill"), fill);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
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

        const octagon_texture = zgl.Texture.create(.@"2d");
        octagon_texture.bindTo(2);
        octagon_texture.parameter(.wrap_s, .clamp_to_border);
        octagon_texture.parameter(.wrap_t, .clamp_to_border);
        octagon_texture.parameter(.min_filter, .linear);
        octagon_texture.parameter(.mag_filter, .linear);
        octagon_texture.storage2D(1, .r8, 64, 64);
        octagon_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, octagon_sdf);
    }
};

pub fn show(context: *Context) !void {
    try glfw.init();
    defer glfw.terminate();

    glfw.windowHint(.ContextVersionMajor, 3);
    glfw.windowHint(.ContextVersionMinor, 3);
    glfw.windowHint(.OpenGLProfile, @enumToInt(glfw.GLProfileAttribute.OpenglCoreProfile));

    const window = try glfw.createWindow(
        @intCast(c_int, window_width),
        @intCast(c_int, window_height),
        "Input Viewer",
        null,
        null,
    );

    glfw.makeContextCurrent(window);

    // wait for vsync to reduce cpu usage
    glfw.swapInterval(1);
    _ = glfw.setFramebufferSizeCallback(window, framebufferSizeCallback);

    const theme = &context.theme;
    const display = Display.init(theme.*);

    while (!glfw.windowShouldClose(window)) {
        const input = blk: {
            const held = context.mutex.acquire();
            defer held.release();

            break :blk context.input;
        };

        zgl.clearColor(
            theme.colors.background[0],
            theme.colors.background[1],
            theme.colors.background[2],
            1.0,
        );
        zgl.clear(.{ .color = true });

        display.draw(input);

        glfw.swapBuffers(window);
        glfw.pollEvents();
    }
}

fn framebufferSizeCallback(_: *glfw.Window, width: i32, height: i32) callconv(.C) void {
    window_width = @intCast(u32, width);
    window_height = @intCast(u32, height);

    zgl.viewport(0, 0, window_width, window_height);
}
