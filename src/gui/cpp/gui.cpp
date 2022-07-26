#include "gui.h"

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <fmt/core.h>
#include <imgui.h>
#include <imgui_internal.h>

#include <cstring>
#include <exception>
#include <filesystem>
#include <fstream>
#include <string>

namespace fs = std::filesystem;

using json = nlohmann::json;

void Gui::drawAndUpdate() {
    ImGuiWindowFlags window_flags = ImGuiWindowFlags_NoDocking | ImGuiWindowFlags_NoBackground |
                                    ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoCollapse |
                                    ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoBringToFrontOnFocus |
                                    ImGuiWindowFlags_NoNavFocus | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_MenuBar |
                                    ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse;

    const ImGuiIO& io = ImGui::GetIO();
    ImGui::SetNextWindowPos(ImVec2(0.0f, 0.0f));
    ImGui::SetNextWindowSize(io.DisplaySize);

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(0.0f, 0.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowBorderSize, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_ChildBorderSize, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 0.0f);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowRounding, 0.0f);
    ImGui::Begin("Content", nullptr, window_flags);
    ImGui::PopStyleVar(5);

    dockspace_id = ImGui::GetID("ContentDockSpace");
    if (ImGui::DockBuilderGetNode(dockspace_id) == nullptr) {
        ImGui::DockBuilderRemoveNode(dockspace_id);                             // Clear out existing layout
        ImGui::DockBuilderAddNode(dockspace_id, ImGuiDockNodeFlags_DockSpace);  // Add empty node
        ImGui::DockBuilderSetNodeSize(dockspace_id, io.DisplaySize);

        ImGuiID dock_main_id = dockspace_id;
        ImGuiID dock_id_left = ImGui::DockBuilderSplitNode(dock_main_id, ImGuiDir_Left, 0.5f, nullptr, &dock_main_id);
        ImGuiID dock_id_left_bottom =
            ImGui::DockBuilderSplitNode(dock_id_left, ImGuiDir_Down, 0.5f, nullptr, &dock_id_left);

        ImGui::DockBuilderDockWindow("Content", dock_main_id);
        ImGui::DockBuilderDockWindow("Calibration Data", dock_id_left);
        ImGui::DockBuilderDockWindow("Log", dock_id_left_bottom);
        ImGui::DockBuilderDockWindow("Config", dock_main_id);
        ImGui::DockBuilderFinish(dockspace_id);
    }

    ImGui::DockSpace(dockspace_id, io.DisplaySize);

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
            ImGui::MenuItem("[DEBUG] Demo Window", nullptr, &draw_demo_window);

            ImGui::EndMenu();
        }

        ImGui::EndMenuBar();
    }

    drawConfigEditor("Config", draw_config);
    drawCalibrationData("Calibration Data", draw_calibration_data);
    log.draw("Log", draw_log);

    // 1. Show the big demo window (Most of the sample code is in
    // ImGui::ShowDemoWindow()! You can browse its code to learn more about
    // Dear ImGui!).
    if (draw_demo_window) ImGui::ShowDemoWindow(&draw_demo_window);

    ImGui::End();
}

void Gui::drawConfigEditor(const char* title, bool& open) {
    if (!open) {
        return;
    }

    if (!ImGui::Begin(title, &open, ImGuiWindowFlags_NoFocusOnAppearing) ||
        feeder_needs_reload.load(std::memory_order_acquire)) {
        ImGui::End();
        return;
    }

    if (!config.has_value()) {
        loadConfig();
    }

    const auto& schema = config_schema;
    auto& config = this->config.value();

    ImGui::Text("Profile");

    ImGui::Separator();

    ImGui::Text("Misc");

    if (ImGui::Button("Reload Config")) {
        loadConfig();
        feeder_needs_reload.store(true, std::memory_order_release);
    }

    ImGui::SameLine();

    bool config_modified = false;

    if (ImGui::Button("Update Schema URL")) {
        config["$schema"] = fmt::format("{}{}{}", context.usercontent_url, '/', context.schema_rel_path_str);
        config_modified = true;
    }

    ImGui::Separator();

    ImGui::Text("Settings");

    const auto& profile_properties =
        schema.at("properties").at("profiles").at("items").at("properties").at("config").at("properties");

    for (const auto& [key, value] : profile_properties.items()) {
        ImGui::TextUnformatted(key.c_str());
    }

    if (config_modified) {
        saveConfig();
        feeder_needs_reload.store(true, std::memory_order_release);
    }

    ImGui::End();
}

void Gui::drawCalibrationData(const char* title, bool& open) {
    if (!open) {
        return;
    }

    if (!ImGui::Begin(title, &open, ImGuiWindowFlags_NoFocusOnAppearing)) {
        ImGui::End();
        return;
    }

    ImGui::End();
}

void Gui::loadConfig() {
    std::ifstream config_file(context.config_path.c_str());
    if (!config_file.is_open()) {
        throw std::runtime_error(fmt::format("Failed to open for reading \"{}\": {}",
                                             context.config_path.string().c_str(), std::strerror(errno)));
    }

    if (config.has_value()) {
        config->clear();
    } else {
        config.emplace();
    }

    config_file >> config.value();
}

void Gui::saveConfig() {
    if (config.has_value()) {
        std::ofstream config_file(context.config_path.c_str());
        if (!config_file.is_open()) {
            throw std::runtime_error(fmt::format("Failed to open for writing \"{}\": {}",
                                                 context.config_path.string().c_str(), std::strerror(errno)));
        }

        config_file << config->dump(4);
    } else {
        log.add("warn: saveConfig(): config was null");
    }
}
