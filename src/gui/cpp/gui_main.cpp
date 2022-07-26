#include <fmt/core.h>
#include <glad/glad.h>

#include <algorithm>
#include <cmath>
#include <filesystem>
#include <nlohmann/json.hpp>
#include <string>
#include <string_view>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <backends/imgui_impl_glfw.h>
#include <backends/imgui_impl_opengl3.h>
#include <imgui.h>

#include "app_log.h"
#include "gui.h"
#include "gui_main.h"

namespace fs = std::filesystem;

using json = nlohmann::json;

static AppLog app_log;

extern "C" void addLogMessage(const char* message) { app_log.add(message); }

extern "C" int runImGui(CUIContext* c_context) {
    if (gladLoadGL() == 0) {
        fmt::print(stderr, "gladLoadGL failed\n");
        return 1;
    }

    UIContext context(*c_context);
    GLFWwindow* window = context.window;

    const fs::path ini_path = context.exe_dir / "imgui-gcfeeder.ini";
    const std::u8string ini_path_str = ini_path.u8string();

    Gui gui(context, app_log, json::parse(context.schema_str));

    // Setup Dear ImGui context
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGuiIO& io = ImGui::GetIO();
    io.IniFilename = reinterpret_cast<const char*>(ini_path_str.c_str());
    // io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;     // Enable
    // Keyboard Controls io.ConfigFlags |= ImGuiConfigFlags_NavEnableGamepad; //
    // Enable Gamepad Controls
    io.ConfigFlags |= ImGuiConfigFlags_DockingEnable;

    float scale = [&]() {
        const auto monitor = glfwGetPrimaryMonitor();
        if (monitor != nullptr) {
            float xscale, yscale;
            glfwGetMonitorContentScale(monitor, &xscale, &yscale);
            return std::max(xscale, yscale);
        } else {
            return 1.0f;
        }
    }();

    // Setup Dear ImGui style
    ImGui::StyleColorsDark();
    ImGuiStyle& style = ImGui::GetStyle();
    style.ScaleAllSizes(scale);
    style.FrameRounding = 2.0f;
    style.WindowRounding = 5.0f;

    // Setup Platform/Renderer backends
    ImGui_ImplGlfw_InitForOpenGL(window, true);
    ImGui_ImplOpenGL3_Init(context.glsl_version.data());

    ImFontConfig font_config;
    font_config.FontDataOwnedByAtlas = false;
    ImFont* font = io.Fonts->AddFontFromMemoryTTF(context.ttf.data(), context.ttf.size(), std::floorf(16.0f * scale),
                                                  &font_config);
    IM_ASSERT(font != nullptr);

    // Our state
    ImVec4 clear_color = ImVec4(0.45f, 0.55f, 0.60f, 1.00f);

    // Main loop
    while (!glfwWindowShouldClose(window)) {
        // Poll and handle events (inputs, window resize, etc.)
        // You can read the io.WantCaptureMouse, io.WantCaptureKeyboard flags to
        // tell if dear imgui wants to use your inputs.
        // - When io.WantCaptureMouse is true, do not dispatch mouse input data
        // to your main application, or clear/overwrite your copy of the mouse
        // data.
        // - When io.WantCaptureKeyboard is true, do not dispatch keyboard input
        // data to your main application, or clear/overwrite your copy of the
        // keyboard data. Generally you may always pass all inputs to dear
        // imgui, and hide them from your application based on those two flags.
        glfwPollEvents();

        // Start the Dear ImGui frame
        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplGlfw_NewFrame();
        ImGui::NewFrame();

        gui.drawAndUpdate();

        // Rendering
        ImGui::Render();
        int display_w, display_h;
        glfwGetFramebufferSize(window, &display_w, &display_h);
        glViewport(0, 0, display_w, display_h);
        glClearColor(clear_color.x * clear_color.w, clear_color.y * clear_color.w, clear_color.z * clear_color.w,
                     clear_color.w);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());

        glfwSwapBuffers(window);
    }

    // Cleanup
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImGui::DestroyContext();

    return 0;
}
