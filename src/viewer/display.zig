const std = @import("std");
const zgl = @import("zgl");
const zlm = @import("zlm");
const glfw = @import("zglfw");
const Input = @import("adapter").Input;
const Calibration = @import("adapter").Calibration;
const Context = @import("root").Context;
const time = std.time;

var window_width: u32 = 512;
var window_height: u32 = 256;

const Display = struct {
    const Which = enum(i32) {
        button_a = 0,
        button_b = 1,
        button_x = 2,
        button_y = 3,
        button_start = 4,
        button_z = 5,
        stick_main = 6,
        stick_c = 7,
        trigger_left = 8,
        trigger_right = 9,
        pad_up = 10,
        pad_left = 11,
        pad_right = 12,
        pad_down = 13,
    };

    const vertex_shader_source: []const u8 = @embedFile("shader/vertex.glsl");
    const background_shader_source: []const u8 = @embedFile("shader/background_fragment.glsl");
    const circle_button_shader_source: []const u8 = @embedFile("shader/circle_button_fragment.glsl");
    const sdf_button_shader_source: []const u8 = @embedFile("shader/sdf_button_fragment.glsl");
    const trigger_shader_source: []const u8 = @embedFile("shader/trigger_fragment.glsl");
    const stick_shader_source: []const u8 = @embedFile("shader/stick_fragment.glsl");
    const default_color_shader_source: []const u8 = @embedFile("shader/color.glsl");

    const bean_sdf = @embedFile("sdf/bean-sdf.gray");
    const z_button_sdf = @embedFile("sdf/z-button-sdf.gray");
    const octagon_sdf = @embedFile("sdf/octagon-sdf.gray");

    const buttons_center = zlm.Mat4.createTranslationXYZ(0.5, -0.075, 0.0);

    background_program: zgl.Program,
    circle_button_program: zgl.Program,
    sdf_button_program: zgl.Program,
    trigger_program: zgl.Program,
    stick_program: zgl.Program,
    vbo: zgl.Buffer,
    vao: zgl.VertexArray,
    ebo: zgl.Buffer,
    timer: time.Timer,

    pub fn init(color_shader_source: ?[]const u8) !Display {
        const vertex_shader = zgl.Shader.create(.vertex);
        defer vertex_shader.delete();
        vertex_shader.source(1, &vertex_shader_source);
        vertex_shader.compile();

        const background_shader = zgl.Shader.create(.fragment);
        defer background_shader.delete();
        background_shader.source(1, &background_shader_source);
        background_shader.compile();

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

        const color_shader = zgl.Shader.create(.fragment);
        defer color_shader.delete();
        color_shader.source(
            1,
            if (color_shader_source) |cs| &cs else &default_color_shader_source,
        );
        color_shader.compile();

        if (zgl.getShader(color_shader, .compile_status) == 0) {
            std.log.err("color_shader compile log: {s}", .{
                color_shader.getCompileLog(std.testing.allocator) catch unreachable,
            });
            @panic("Failed to compile color_shader");
        }

        const background_program = zgl.Program.create();
        background_program.attach(vertex_shader);
        background_program.attach(background_shader);
        background_program.attach(color_shader);
        background_program.link();

        const circle_button_program = zgl.Program.create();
        circle_button_program.attach(vertex_shader);
        circle_button_program.attach(circle_button_shader);
        circle_button_program.attach(color_shader);
        circle_button_program.link();

        const sdf_button_program = zgl.Program.create();
        sdf_button_program.attach(vertex_shader);
        sdf_button_program.attach(sdf_button_shader);
        sdf_button_program.attach(color_shader);
        sdf_button_program.link();

        const trigger_program = zgl.Program.create();
        trigger_program.attach(vertex_shader);
        trigger_program.attach(trigger_shader);
        trigger_program.attach(color_shader);
        trigger_program.link();

        const stick_program = zgl.Program.create();
        stick_program.attach(vertex_shader);
        stick_program.attach(stick_shader);
        stick_program.attach(color_shader);
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
            .background_program = background_program,
            .circle_button_program = circle_button_program,
            .sdf_button_program = sdf_button_program,
            .trigger_program = trigger_program,
            .stick_program = stick_program,
            .vbo = vbo,
            .vao = vao,
            .ebo = ebo,
            .timer = try time.Timer.start(),
        };
    }

    pub fn draw(self: Display, input: ?Input) void {
        self.vao.bind();

        const width = @intToFloat(f32, window_width);
        const height = @intToFloat(f32, window_height);
        const aspect = width / height;
        const projection = if (window_width >= window_height)
            zlm.Mat4.createOrthogonal(-0.5 * aspect, 0.5 * aspect, -0.5, 0.5, -1.0, 1.0)
        else
            zlm.Mat4.createOrthogonal(-0.5, 0.5, -0.5 / aspect, 0.5 / aspect, -1.0, 1.0);

        const programs = [_]zgl.Program{
            self.background_program,
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

            zgl.uniform2f(program.uniformLocation("resolution"), width, height);
            zgl.uniform1f(
                program.uniformLocation("time"),
                @intToFloat(f32, self.timer.read()) / @intToFloat(f32, time.ns_per_s),
            );
        }

        self.drawBackground();
        self.drawCircleButtons(input);
        self.drawSdfButtons(input);
        self.drawSticks(input);
        self.drawTriggers(input);
        self.drawDpad(input);
    }

    fn drawBackground(self: Display) void {
        const program = self.background_program;
        program.use();
        const model = zlm.Mat4.createUniformScale(2.0);
        program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
        zgl.drawElements(.triangles, 6, .u32, 0);
    }

    fn drawCircleButtons(self: Display, input: ?Input) void {
        const program = self.circle_button_program;
        program.use();
        // a button
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_a));
            const scale = 1.5;
            const model = zlm.Mat4.createUniformScale(scale).mul(buttons_center);
            program.uniform1f(program.uniformLocation("scale"), scale);

            // use programUniform1i instead because uniform1i has a name conflict
            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_a else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // b button
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_b));
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
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // start button
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_start));
            const scale = 0.625;
            const model = zlm.Mat4.createUniformScale(scale).mul(buttons_center).mul(
                zlm.Mat4.createTranslationXYZ(-0.325, 0.05, 0.0),
            );
            program.uniform1f(program.uniformLocation("scale"), scale);

            zgl.programUniform1i(
                program,
                program.uniformLocation("pressed"),
                @boolToInt(if (input) |in| in.button_start else false),
            );
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawSdfButtons(self: Display, input: ?Input) void {
        const bean_scale = 0.275;
        const bean_scale_mat = zlm.Mat4.createUniformScale(bean_scale);

        const program = self.sdf_button_program;
        program.use();
        zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 0);
        program.uniform1f(program.uniformLocation("scale"), bean_scale);
        // y button
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_y));
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
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_x));
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
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.button_z));
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
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawSticks(self: Display, input: ?Input) void {
        const scale = 0.6;
        const scale_mat = zlm.Mat4.createUniformScale(scale);

        const program = self.stick_program;
        program.use();
        zgl.programUniform1i(program, program.uniformLocation("sdf_texture"), 2);
        program.uniform1f(program.uniformLocation("scale"), scale);
        // main stick
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.stick_main));
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

            zgl.uniform2f(program.uniformLocation("pos"), x, y);

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // c stick
        {
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.stick_c));
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

            zgl.uniform2f(program.uniformLocation("pos"), x, y);

            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn drawTriggers(self: Display, input: ?Input) void {
        const scale = 0.375;
        const scale_mat = zlm.Mat4.createUniformScale(scale);

        const program = self.trigger_program;
        program.use();
        program.uniform1f(program.uniformLocation("scale"), scale);
        // left trigger
        {
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_l else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.trigger_left));
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
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_r else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.trigger_right));
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

    fn drawDpad(self: Display, input: ?Input) void {
        const program = self.circle_button_program;
        program.use();
        const center = zlm.Vec3.new(-0.4, -0.3, 0.0);
        const center_translate = zlm.Mat4.createTranslation(center);
        const button_translate = zlm.Mat4.createTranslationXYZ(0.0, 0.095, 0.0);
        const scale = 0.55;
        // up
        {
            const model = zlm.Mat4.createUniformScale(scale).mul(button_translate).mul(center_translate);
            program.uniform1f(program.uniformLocation("scale"), scale);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_up else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.pad_up));
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // left
        {
            const model = zlm.Mat4.createUniformScale(scale).mul(button_translate).mul(
                zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(-90.0)),
            ).mul(center_translate);
            program.uniform1f(program.uniformLocation("scale"), scale);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_left else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.pad_up));
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // right
        {
            const model = zlm.Mat4.createUniformScale(scale).mul(button_translate).mul(
                zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(90.0)),
            ).mul(center_translate);
            program.uniform1f(program.uniformLocation("scale"), scale);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_right else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.pad_up));
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
        // down
        {
            const model = zlm.Mat4.createUniformScale(scale).mul(button_translate).mul(
                zlm.Mat4.createAngleAxis(zlm.Vec3.unitZ, zlm.toRadians(180.0)),
            ).mul(center_translate);
            program.uniform1f(program.uniformLocation("scale"), scale);
            program.uniformMatrix4(program.uniformLocation("model"), false, &[_][4][4]f32{model.fields});
            zgl.uniform1i(program.uniformLocation("pressed"), @boolToInt(if (input) |in| in.button_down else false));
            zgl.uniform1i(program.uniformLocation("which"), @enumToInt(Which.pad_up));
            zgl.drawElements(.triangles, 6, .u32, 0);
        }
    }

    fn loadTextures() void {
        const bean_texture = zgl.genTexture();
        zgl.activeTexture(.texture_0);
        zgl.bindTexture(bean_texture, .@"2d");
        bean_texture.parameter(.wrap_s, .clamp_to_border);
        bean_texture.parameter(.wrap_t, .clamp_to_border);
        bean_texture.parameter(.min_filter, .linear);
        bean_texture.parameter(.mag_filter, .linear);
        bean_texture.storage2D(1, .r8, 64, 64);
        bean_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, bean_sdf);

        const z_button_texture = zgl.genTexture();
        zgl.activeTexture(.texture_1);
        zgl.bindTexture(z_button_texture, .@"2d");
        z_button_texture.parameter(.wrap_s, .clamp_to_border);
        z_button_texture.parameter(.wrap_t, .clamp_to_border);
        z_button_texture.parameter(.min_filter, .linear);
        z_button_texture.parameter(.mag_filter, .linear);
        z_button_texture.storage2D(1, .r8, 64, 64);
        z_button_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, z_button_sdf);

        const octagon_texture = zgl.genTexture();
        zgl.activeTexture(.texture_2);
        zgl.bindTexture(octagon_texture, .@"2d");
        octagon_texture.parameter(.wrap_s, .clamp_to_border);
        octagon_texture.parameter(.wrap_t, .clamp_to_border);
        octagon_texture.parameter(.min_filter, .linear);
        octagon_texture.parameter(.mag_filter, .linear);
        octagon_texture.storage2D(1, .r8, 64, 64);
        octagon_texture.subImage2D(0, 0, 0, 64, 64, .red, .unsigned_byte, octagon_sdf);

        // Bind active texture to unused unit to workaround Discord screen
        // share bug.
        zgl.activeTexture(.texture_3);
    }
};

pub fn show(context: *Context, color_shader_source: ?[]const u8) !void {
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

    const display = try Display.init(color_shader_source);

    while (!glfw.windowShouldClose(window)) {
        const input = blk: {
            const held = context.mutex.acquire();
            defer held.release();

            break :blk context.input;
        };

        zgl.clearColor(0.0, 0.0, 0.0, 1.0);
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
