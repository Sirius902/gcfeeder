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
#include <string_view>

#include "util.h"

namespace fs = std::filesystem;

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

    ImGuiWindowFlags flags = ImGuiWindowFlags_NoFocusOnAppearing;
    if (editor_profile_dirty) flags |= ImGuiWindowFlags_UnsavedDocument;

    if (!ImGui::Begin(title, &open, flags) || feeder_needs_reload.load(std::memory_order_acquire)) {
        ImGui::End();
        return;
    }

    if (!config.has_value()) {
        loadConfig();
    } else if (scheduled_reload) {
        loadConfig();
        feeder_needs_reload.store(true, std::memory_order_release);
        editor_profile.reset();
        editor_profile_dirty = false;
        scheduled_reload = false;
    }

    constexpr auto header_color = ImVec4(0x61 / 255.0f, 0x8C / 255.0f, 0xCA / 255.0f, 1.0f);

    const auto& schema = config_schema;
    auto& config = this->config.value();

    bool config_modified = false;

    ImGui::TextColored(header_color, "Profiles");
    ImGui::Spacing();

    auto& profiles = config.at("profiles");

    auto& current_profile_name = config.at("current_profile").get_ref<std::string&>();
    ImGui::Text("Current");
    ImGui::SameLine();
    if (ImGui::BeginCombo("##combo", current_profile_name.c_str())) {
        for (const auto& [_, profile] : profiles.items()) {
            const auto& name = profile.at("name").get_ref<const std::string&>();
            bool selected = name == current_profile_name;

            if (ImGui::Selectable(name.c_str(), selected)) {
                current_profile_name = name;
                config_modified = true;
                editor_profile.reset();
                editor_profile_dirty = false;
            }

            if (selected) ImGui::SetItemDefaultFocus();
        }

        ImGui::EndCombo();
    }

    ImGui::Separator();

    ImGui::TextColored(header_color, "Misc");
    ImGui::Spacing();

    if (ImGui::Button("Reload Config")) {
        scheduled_reload = true;
    }

    ImGui::SameLine();

    if (ImGui::Button("Update Schema URL")) {
        config["$schema"] = fmt::format("{}{}{}", context.usercontent_url, '/', context.schema_rel_path_str);
        config_modified = true;
    }

    ImGui::Separator();

    ImGui::TextColored(header_color, "Settings");
    ImGui::Spacing();

    bool save_profile = ImGui::Button("Save Changes");
    ImGui::SameLine();
    if (ImGui::Button("Discard Changes")) {
        editor_profile.reset();
        editor_profile_dirty = false;
    }
    ImGui::Spacing();

    const auto& profile_properties =
        schema.at("properties").at("profiles").at("items").at("properties").at("config").at("properties");

    auto& current_profile = [&]() -> json& {
        if (!editor_profile.has_value()) {
            for (auto& [_, value] : config.at("profiles").items()) {
                if (value.at("name").get_ref<const std::string&>() == current_profile_name) {
                    editor_profile = value.at("config");
                    break;
                }
            }

            if (!editor_profile.has_value()) {
                // TODO: Don't crash if profile is not in list.
                throw std::runtime_error(fmt::format("profile not found: \"{}\"", current_profile_name));
            }
        }

        return editor_profile.value();
    }();

    ImGui::PushItemWidth(0.55f * ImGui::GetWindowWidth());
    drawConfigEditorObject(profile_properties, current_profile);
    ImGui::PopItemWidth();

    if (save_profile) {
        // TODO: Extract to member function
        bool found = false;
        for (auto& [_, value] : config.at("profiles").items()) {
            if (value.at("name").get_ref<const std::string&>() == current_profile_name) {
                value.at("config") = current_profile;
                found = true;
                break;
            }
        }
        if (!found) {
            throw std::runtime_error(fmt::format("profile not found: \"{}\"", current_profile_name));
        }
        editor_profile_dirty = false;
        config_modified = true;
    }

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

        if (auto it = value.find("anyOf"); it != value.end()) {
            ImGui::SetNextItemOpen(true);
            if (ImGui::TreeNode(key.c_str())) {
                addDescription();

                ImGui::TextUnformatted("Variant");
                ImGui::SameLine();
                // TODO: null is considered an object by nlohmann_json. Currently reports object instead of null.
                if (ImGui::BeginCombo("##combo", data.type_name())) {
                    for (const auto& [_, variant] : it->items()) {
                        const auto type = variant.at("type").get_ref<const std::string&>();
                        bool selected = type == data.type_name();

                        if (ImGui::Selectable(type.c_str(), selected)) {
                            editor_profile_dirty = true;
                        }

                        if (selected) ImGui::SetItemDefaultFocus();
                    }

                    ImGui::EndCombo();
                }

                // TODO: Render value from config based on variant.

                ImGui::TreePop();
            } else {
                addDescription();
            }
        } else if (auto it = value.find("type"); it != value.end()) {
            const auto& type = it->get_ref<const std::string&>();

            if (type == "object") {
                ImGui::SetNextItemOpen(true);
                if (ImGui::TreeNode(key.c_str())) {
                    addDescription();
                    drawConfigEditorObject(value.at("properties"), data.at(key));
                    ImGui::TreePop();
                } else {
                    addDescription();
                }
            } else if (type == "boolean") {
                if (ImGui::Checkbox(key.c_str(), data.at(key).get_ptr<bool*>())) {
                    editor_profile_dirty = true;
                }
                addDescription();
            } else if (type == "integer") {
                auto& field = data.at(key).get_ref<json::number_integer_t&>();
                const auto minimum = value.find("minimum");
                const auto maximum = value.find("maximum");

                // TODO: Complain if field is too large.
                int v = util::lossy_cast<int>(field);
                if (ImGui::InputInt(key.c_str(), &v)) {
                    field = static_cast<json::number_integer_t>(v);
                    if (minimum != value.end()) {
                        field = std::max(field, minimum->get<json::number_integer_t>());
                    }
                    if (maximum != value.end()) {
                        field = std::min(field, maximum->get<json::number_integer_t>());
                    }

                    editor_profile_dirty = true;
                }
                addDescription();
            } else if (type == "number") {
                auto& field = data.at(key).get_ref<json::number_float_t&>();
                const auto minimum = value.find("minimum");
                const auto maximum = value.find("maximum");

                double v = util::lossy_cast<double>(field);
                if (ImGui::InputDouble(key.c_str(), &v, 0.1, 0.0, "%.2f")) {
                    field = static_cast<json::number_float_t>(v);
                    if (minimum != value.end()) {
                        field = std::max(field, minimum->get<json::number_float_t>());
                    }
                    if (maximum != value.end()) {
                        field = std::min(field, maximum->get<json::number_float_t>());
                    }
                    editor_profile_dirty = true;
                }
                addDescription();
            } else if (type == "string") {
                const auto variants = value.find("enum");
                if (variants == value.end()) {
                    warningText(fmt::format("{}: non-enum strings unsupported", key).c_str());
                    continue;
                }

                auto& current_variant = data.at(key).get_ref<std::string&>();
                if (ImGui::BeginCombo(key.c_str(), current_variant.c_str())) {
                    for (const auto& [_, variant_obj] : variants->items()) {
                        const auto& variant = variant_obj.get_ref<const std::string&>();
                        bool selected = variant == current_variant;

                        if (ImGui::Selectable(variant.c_str(), selected)) {
                            current_variant = variant;
                            editor_profile_dirty = true;
                        }

                        if (selected) ImGui::SetItemDefaultFocus();
                    }

                    ImGui::EndCombo();
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

using FormatCallback = std::size_t (*)(const char* bytes_ptr, std::size_t bytes_len, void* userdata);
extern "C" void formatConfigJson(const char* json_ptr, std::size_t json_len, FormatCallback callback, void* userdata);

void Gui::saveConfig() {
    if (config.has_value()) {
        std::ofstream config_file(context.config_path.c_str());
        if (!config_file.is_open()) {
            throw std::runtime_error(fmt::format("Failed to open for writing \"{}\": {}",
                                                 context.config_path.string().c_str(), std::strerror(errno)));
        }

        const auto dump = config->dump();
        formatConfigJson(
            dump.c_str(), dump.size(),
            [](const char* bytes_ptr, std::size_t bytes_len, void* userdata) {
                std::ofstream& config_file = *static_cast<std::ofstream*>(userdata);
                config_file << std::string_view{bytes_ptr, bytes_len};
                return bytes_len;
            },
            static_cast<void*>(&config_file));
    } else {
        log.add("warn: saveConfig(): config was null");
    }
}
