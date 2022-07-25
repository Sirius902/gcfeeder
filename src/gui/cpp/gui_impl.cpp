#include "gui_impl.h"

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

void Gui::drawAndUpdate(UIContext& context) {
    const ImGuiIO& io = ImGui::GetIO();
    ImGui::SetNextWindowPos(ImVec2(0.0f, 0.0f));
    ImGui::SetNextWindowSize(io.DisplaySize);

    ImGuiWindowFlags window_flags = ImGuiWindowFlags_NoDocking | ImGuiWindowFlags_NoBackground |
                                    ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoCollapse |
                                    ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoBringToFrontOnFocus |
                                    ImGuiWindowFlags_NoNavFocus | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_MenuBar;

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(0.0f, 0.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_ChildBorderSize, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 0.0f);
    ImGui::Begin("Content", nullptr, window_flags);
    ImGui::PopStyleVar(5);

    if (ImGui::BeginMenuBar()) {
        if (ImGui::BeginMenu("File")) {
            if (ImGui::MenuItem("Minimize")) {
                // TOOD: Implement.
            }

            if (ImGui::MenuItem("Exit", "Alt+F4")) {
                glfwSetWindowShouldClose(context.window, true);
            }

            ImGui::EndMenu();
        }

        if (ImGui::BeginMenu("Calibrate")) {
            if (ImGui::MenuItem("PC Scaling")) {
                // TOOD: Implement.
            }

            if (ImGui::MenuItem("ESS Adapter")) {
                // TOOD: Implement.
            }

            ImGui::EndMenu();
        }

        if (ImGui::BeginMenu("View")) {
            ImGui::MenuItem("Config", nullptr, &draw_config);
            ImGui::MenuItem("Calibration Data", nullptr, &draw_calibration_data);
            ImGui::MenuItem("Log", nullptr, &draw_log);
            ImGui::MenuItem("DEBUG: Demo Window", nullptr, &draw_demo_window);

            ImGui::EndMenu();
        }

        ImGui::EndMenuBar();
    }

    if (draw_config) {
        ImGui::SetNextWindowPos(ImVec2(410.0f, 30.0f), ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(400.0f, 505.0f), ImGuiCond_FirstUseEver);
        if (!ImGui::Begin("Config", &draw_config, ImGuiWindowFlags_NoFocusOnAppearing)) {
            ImGui::End();
        } else {
            ImGui::Text("Profile");

            ImGui::Separator();

            ImGui::Text("Misc");

            bool update_schema = ImGui::Button("Update Schema URL");
            (void)update_schema;

            ImGui::Separator();

            ImGui::Text("Settings");

            ImGui::Checkbox("Rumble Enabled", &rumble_enabled);

            ImGui::End();
        }
    }

    if (draw_calibration_data) {
        ImGui::SetNextWindowPos(ImVec2(5.0f, 285.0f), ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(400.0f, 250.0f), ImGuiCond_FirstUseEver);
        if (!ImGui::Begin("Calibration Data", &draw_config, ImGuiWindowFlags_NoFocusOnAppearing)) {
            ImGui::End();
        } else {
            ImGui::End();
        }
    }

    log.draw("Log", draw_log);

    // 1. Show the big demo window (Most of the sample code is in
    // ImGui::ShowDemoWindow()! You can browse its code to learn more about
    // Dear ImGui!).
    if (draw_demo_window) ImGui::ShowDemoWindow(&draw_demo_window);

    ImGui::End();
}
