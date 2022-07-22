const std = @import("std");
const builtin = @import("builtin");
const imgui = @import("imgui");
const impl_glfw = @import("imgui_impl_glfw.zig");
const impl_gl3 = @import("imgui_impl_opengl3.zig");
const glfw = @import("glfw.zig");
const gl = @import("gl.zig");

const roboto_ttf: []u8 = blk: {
    const ttf_const = @embedFile("font/Roboto-Medium.ttf");
    var buf: [ttf_const.len]u8 = undefined;
    @memcpy(&buf, ttf_const, buf.len);
    break :blk &buf;
};

const is_darwin = builtin.os.tag.isDarwin();

var draw_log = true;
fn drawGui() !void {
    const viewport = imgui.GetMainViewport().?;
    {
        const extra_pixels = 4;
        imgui.SetNextWindowPos(.{ .x = viewport.WorkPos.x - @divTrunc(extra_pixels, 2), .y = viewport.WorkPos.y });
        imgui.SetNextWindowSize(.{ .x = viewport.WorkSize.x + extra_pixels, .y = viewport.WorkSize.y });
    }

    _ = imgui.BeginExt("gcfeeder", null, .{
        .NoBackground = true,
        .NoTitleBar = true,
        .NoCollapse = true,
        .NoMove = true,
        .MenuBar = true,
        .NoBringToFrontOnFocus = true,
        .NoNavFocus = true,
        .NoResize = true,
    });

    if (imgui.BeginMenuBar()) {
        if (imgui.BeginMenu("View")) {
            _ = imgui.MenuItem_BoolPtr("Log", null, &draw_log);

            imgui.EndMenu();
        }

        imgui.EndMenuBar();
    }

    drawLog(&draw_log);

    imgui.End();
}

var log_auto_scroll = true;
fn drawLog(open: *bool) void {
    if (!open.*) {
        return;
    }

    imgui.SetNextWindowSizeExt(.{ .x = 400.0, .y = 250.0 }, .{ .Once = true });
    if (!imgui.BeginExt("Log", open, .{ .NoFocusOnAppearing = true })) {
        imgui.End();
        return;
    }

    if (imgui.BeginPopup("Options")) {
        _ = imgui.Checkbox("Auto-scroll", &log_auto_scroll);
        imgui.EndPopup();
    }

    if (imgui.Button("Options")) {
        imgui.OpenPopup_Str("Options");
    }
    imgui.SameLine();
    const clear = imgui.Button("Clear");
    imgui.SameLine();
    const copy = imgui.Button("Copy");

    imgui.Separator();

    if (clear) {
        std.log.debug("clear", .{});
    }

    if (copy) {
        std.log.debug("copy", .{});
    }

    var i: usize = 0;
    while (i < 30) : (i += 1) {
        imgui.TextUnformatted("info: Hello world!");
    }

    if (log_auto_scroll and imgui.GetScrollY() >= imgui.GetScrollMaxY()) {
        imgui.SetScrollHereYExt(1.0);
    }

    imgui.End();
}

pub fn runImGui(allocator: std.mem.Allocator) !void {
    const ini_path = blk: {
        const exe_dir_path = try std.fs.selfExeDirPathAlloc(allocator);
        defer allocator.free(exe_dir_path);

        break :blk try std.mem.joinZ(allocator, "/", &[_][]const u8{ exe_dir_path, "imgui-gcfeeder.ini" });
    };
    defer allocator.free(ini_path);

    // Setup window
    _ = glfw.glfwSetErrorCallback(glfwErrorCallback);
    if (glfw.glfwInit() == 0)
        return error.GlfwInitFailed;

    // Decide GL+GLSL versions
    const glsl_version = if (is_darwin) "#version 150" else "#version 130";
    if (is_darwin) {
        // GL 3.2 + GLSL 150
        glfw.glfwWindowHint(glfw.GLFW_CONTEXT_VERSION_MAJOR, 3);
        glfw.glfwWindowHint(glfw.GLFW_CONTEXT_VERSION_MINOR, 2);
        glfw.glfwWindowHint(glfw.GLFW_OPENGL_PROFILE, glfw.GLFW_OPENGL_CORE_PROFILE); // 3.2+ only
        glfw.glfwWindowHint(glfw.GLFW_OPENGL_FORWARD_COMPAT, gl.GL_TRUE); // Required on Mac
    } else {
        // GL 3.0 + GLSL 130
        glfw.glfwWindowHint(glfw.GLFW_CONTEXT_VERSION_MAJOR, 3);
        glfw.glfwWindowHint(glfw.GLFW_CONTEXT_VERSION_MINOR, 0);
        //glfw.glfwWindowHint(glfw.GLFW_OPENGL_PROFILE, glfw.GLFW_OPENGL_CORE_PROFILE);  // 3.2+ only
        //glfw.glfwWindowHint(glfw.GLFW_OPENGL_FORWARD_COMPAT, gl.GL_TRUE);            // 3.0+ only
    }

    // Create window with graphics context
    const window = glfw.glfwCreateWindow(1280, 720, "gcfeeder", null, null) orelse return error.GlfwCreateWindowFailed;
    glfw.glfwMakeContextCurrent(window);
    glfw.glfwSwapInterval(1); // Enable vsync

    // Setup Dear ImGui context
    imgui.CHECKVERSION();
    _ = imgui.CreateContext();
    const io = imgui.GetIO();
    io.IniFilename = ini_path;
    //io.ConfigFlags |= imgui.ConfigFlags.NavEnableKeyboard;     // Enable Keyboard Controls
    //io.ConfigFlags |= imgui.ConfigFlags.NavEnableGamepad;      // Enable Gamepad Controls

    const scale = blk: {
        if (glfw.glfwGetPrimaryMonitor()) |monitor| {
            var xscale: f32 = undefined;
            var yscale: f32 = undefined;
            glfw.glfwGetMonitorContentScale(monitor, &xscale, &yscale);
            break :blk std.math.max(xscale, yscale);
        } else {
            break :blk 1.0;
        }
    };

    // Setup Dear ImGui style
    imgui.StyleColorsDark();
    {
        const style = imgui.GetStyle().?;
        style.ScaleAllSizes(scale);
        style.FrameRounding = 2.0;
        style.WindowRounding = 5.0;
    }

    // Setup Platform/Renderer bindings
    _ = impl_glfw.InitForOpenGL(window, true);
    _ = impl_gl3.Init(glsl_version);

    // Load Fonts
    // - If no fonts are loaded, dear imgui will use the default font. You can also load multiple fonts and use ImGui::PushFont()/PopFont() to select them.
    // - AddFontFromFileTTF() will return the ImFont* so you can store it if you need to select the font among multiple.
    // - If the file cannot be loaded, the function will return NULL. Please handle those errors in your application (e.g. use an assertion, or display an error and quit).
    // - The fonts will be rasterized at a given size (w/ oversampling) and stored into a texture when calling ImFontAtlas::Build()/GetTexDataAsXXXX(), which ImGui_ImplXXXX_NewFrame below will call.
    // - Read 'docs/FONTS.txt' for more instructions and details.
    // - Remember that in C/C++ if you want to include a backslash \ in a string literal you need to write a double backslash \\ !
    //io.Fonts.AddFontDefault();
    //io.Fonts.AddFontFromFileTTF("../../misc/fonts/Roboto-Medium.ttf", 16.0);
    //io.Fonts.AddFontFromFileTTF("../../misc/fonts/Cousine-Regular.ttf", 15.0);
    //io.Fonts.AddFontFromFileTTF("../../misc/fonts/DroidSans.ttf", 16.0);
    //io.Fonts.AddFontFromFileTTF("../../misc/fonts/ProggyTiny.ttf", 10.0);
    //ImFont* font = io.Fonts.AddFontFromFileTTF("c:\\Windows\\Fonts\\ArialUni.ttf", 18.0, null, io.Fonts->GetGlyphRangesJapanese());
    //IM_ASSERT(font != NULL);

    var font_cfg: imgui.FontConfig = undefined;
    imgui.FontConfig.init_ImFontConfig(&font_cfg);
    defer imgui.FontConfig.deinit(&font_cfg);

    font_cfg.FontDataOwnedByAtlas = false;
    _ = io.Fonts.?.AddFontFromMemoryTTFExt(
        roboto_ttf.ptr,
        @intCast(i32, roboto_ttf.len),
        std.math.floor(16.0 * scale),
        &font_cfg,
        null,
    );

    // Our state
    var show_demo_window = true;
    var clear_color = imgui.Vec4{ .x = 0.45, .y = 0.55, .z = 0.60, .w = 1.00 };

    // Main loop
    while (glfw.glfwWindowShouldClose(window) == 0) {
        // Poll and handle events (inputs, window resize, etc.)
        // You can read the io.WantCaptureMouse, io.WantCaptureKeyboard flags to tell if dear imgui wants to use your inputs.
        // - When io.WantCaptureMouse is true, do not dispatch mouse input data to your main application.
        // - When io.WantCaptureKeyboard is true, do not dispatch keyboard input data to your main application.
        // Generally you may always pass all inputs to dear imgui, and hide them from your application based on those two flags.
        glfw.glfwPollEvents();

        // Start the Dear ImGui frame
        impl_gl3.NewFrame();
        impl_glfw.NewFrame();
        imgui.NewFrame();

        if (show_demo_window)
            imgui.ShowDemoWindowExt(&show_demo_window);

        try drawGui();

        // Rendering
        imgui.Render();
        var display_w: c_int = 0;
        var display_h: c_int = 0;
        glfw.glfwGetFramebufferSize(window, &display_w, &display_h);
        gl.glViewport(0, 0, display_w, display_h);
        gl.glClearColor(
            clear_color.x * clear_color.w,
            clear_color.y * clear_color.w,
            clear_color.z * clear_color.w,
            clear_color.w,
        );
        gl.glClear(gl.GL_COLOR_BUFFER_BIT);
        impl_gl3.RenderDrawData(imgui.GetDrawData());

        glfw.glfwSwapBuffers(window);
    }

    // Cleanup
    impl_gl3.Shutdown();
    impl_glfw.Shutdown();
    imgui.DestroyContext();

    glfw.glfwDestroyWindow(window);
    glfw.glfwTerminate();
}

fn glfwErrorCallback(err: c_int, description: ?[*:0]const u8) callconv(.C) void {
    std.debug.print("Glfw Error {}: {s}\n", .{ err, description });
}
