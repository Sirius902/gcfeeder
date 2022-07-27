#include "gui.h"

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <fmt/core.h>
#include <imgui.h>
#include <imgui_internal.h>

#include <cstddef>
#include <cstring>
#include <exception>
#include <filesystem>
#include <fstream>
#include <string>

#include "util.h"

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

    constexpr auto header_color = ImVec4(0x61 / 255.0f, 0x8C / 255.0f, 0xCA / 255.0f, 1.0f);

    const auto& schema = config_schema;
    auto& config = this->config.value();

    ImGui::TextColored(header_color, "Profiles");
    ImGui::Spacing();

    auto& profiles = config.at("profiles");

    const auto current_profile_name = config.at("current_profile").get_ref<const std::string&>();
    ImGui::Text("Current");
    ImGui::SameLine();
    if (ImGui::BeginCombo("##combo", current_profile_name.c_str())) {
        for (const auto& [_, profile] : profiles.items()) {
            const auto name = profile.at("name").get_ref<const std::string&>();
            bool selected = name == current_profile_name;

            if (ImGui::Selectable(name.c_str(), selected)) {
                fmt::print(stderr, "Selected \"{}\"!\n", name);
            }

            if (selected) ImGui::SetItemDefaultFocus();
        }

        ImGui::EndCombo();
    }

    ImGui::Separator();

    ImGui::TextColored(header_color, "Misc");
    ImGui::Spacing();

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

    ImGui::TextColored(header_color, "Settings");
    ImGui::Spacing();

    const auto& profile_properties =
        schema.at("properties").at("profiles").at("items").at("properties").at("config").at("properties");

    auto& current_profile = [&]() -> json& {
        for (auto& [_, value] : config.at("profiles").items()) {
            if (value.at("name").get_ref<const std::string&>() == current_profile_name) {
                return value.at("config");
            }
        }

        // TODO: Don't crash if profile is not in list.
        throw std::runtime_error(fmt::format("profile not found: \"{}\"", current_profile_name));
    }();
    drawConfigEditorObject(profile_properties, current_profile);

    if (config_modified) {
        saveConfig();
        feeder_needs_reload.store(true, std::memory_order_release);
    }

    // TODO: Workaround for scroll bar being partially off screen. Properly fix.
    ImGui::TextUnformatted("\n");

    ImGui::End();
}

void Gui::drawConfigEditorObject(const json& properties, json& data) {
    for (const auto& [key, value] : properties.items()) {
        const auto& description = [](const json& value) {
            if (auto it = value.find("description"); it != value.end()) {
                return it->get_ref<const std::string&>();
            } else {
                return std::string{};
            }
        }(value);

        const auto addDescription = [&]() {
            if (!description.empty()) {
                ImGui::SameLine();
                ImGui::TextDisabled("(?)");
                if (ImGui::IsItemHovered()) {
                    ImGui::BeginTooltip();
                    ImGui::PushTextWrapPos(ImGui::GetFontSize() * 35.0f);
                    ImGui::TextUnformatted(description.c_str());
                    ImGui::PopTextWrapPos();
                    ImGui::EndTooltip();
                }
            }
        };

        const auto warningText = [](const char* text) {
            constexpr auto warn_color = ImVec4(0.9f, 0.2f, 0.2f, 1.0f);
            ImGui::PushStyleColor(ImGuiCol_Text, warn_color);
            ImGui::TextUnformatted(text);
            ImGui::PopStyleColor();
        };

        if (value.contains("anyOf")) {
            const auto warning = fmt::format("Unimplemented json anyOf: {}\n", key);
            ImGui::TextUnformatted(key.c_str());
            addDescription();
            warningText(warning.c_str());
        } else if (auto it = value.find("type"); it != value.end()) {
            const auto type = it->get_ref<const std::string&>();

            if (type == "object") {
                if (ImGui::TreeNode(key.c_str())) {
                    addDescription();
                    drawConfigEditorObject(value.at("properties"), data.at(key));
                    ImGui::TreePop();
                } else {
                    addDescription();
                }
            } else if (type == "boolean") {
                if (ImGui::Checkbox(key.c_str(), data.at(key).get_ptr<bool*>())) {
                    fmt::print(stderr, "Toggled checkbox :D\n");
                }
                addDescription();
            } else if (type == "integer") {
                auto& field = data.at(key).get_ref<std::int64_t&>();
                const auto minimum = value.find("minimum");
                const auto maximum = value.find("maximum");

                // TODO: Complain if field is too large.
                int v = util::lossy_cast<int>(field);
                if (ImGui::InputInt(key.c_str(), &v)) {
                    field = static_cast<std::int64_t>(v);
                    if (minimum != value.end()) {
                        field = std::max(field, minimum->get<std::int64_t>());
                    }
                    if (maximum != value.end()) {
                        field = std::min(field, maximum->get<std::int64_t>());
                    }

                    fmt::print(stderr, "Inputted integer text! :D\n");
                }
                addDescription();
            } else {
                const auto warning = fmt::format("Unimplemented json type: {}\n", type);
                ImGui::TextUnformatted(key.c_str());
                addDescription();
                warningText(warning.c_str());
            }
        } else {
            const auto warning = fmt::format("Unknown json property with name: {}\n", key);
            ImGui::TextUnformatted(key.c_str());
            addDescription();
            warningText(warning.c_str());
        }
    }
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
