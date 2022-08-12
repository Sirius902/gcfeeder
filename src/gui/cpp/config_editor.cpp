#include "config_editor.h"

#include <fmt/core.h>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>
#include <misc/cpp/imgui_stdlib.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <exception>
#include <numbers>
#include <utility>

#include "util.h"

using json = Config::json;

static void drawWarningText(const char* text) {
    constexpr auto warn_color = ImVec4(0.9f, 0.2f, 0.2f, 1.0f);
    ImGui::PushStyleColor(ImGuiCol_Text, warn_color);
    ImGui::TextUnformatted(text);
    ImGui::PopStyleColor();
}

void ConfigEditor::drawAndUpdate(const char* title, bool& open) {
    if (!open) {
        return;
    }

    ImGuiWindowFlags flags = ImGuiWindowFlags_NoFocusOnAppearing;
    if (profile_dirty) flags |= ImGuiWindowFlags_UnsavedDocument;

    if (!ImGui::Begin(title, &open, flags) || state.feeder_needs_reload.load(std::memory_order_acquire)) {
        ImGui::End();
        return;
    }

    if (!state.config.isLoaded()) {
        state.config.load();
    } else if (scheduled_reload) {
        state.config.load();
        state.feeder_needs_reload.store(true, std::memory_order_release);
        profile.reset();
        profile_dirty = false;
        scheduled_reload = false;
    }

    constexpr auto header_color = ImVec4(0x61 / 255.0f, 0x8C / 255.0f, 0xCA / 255.0f, 1.0f);

    auto& config = state.config.getJson();
    auto& profiles = config.at("profiles").get_ref<json::array_t&>();

    bool config_modified = false;

    ImGui::TextColored(header_color, "Profiles");
    ImGui::Spacing();

    auto& current_profile_name = config.at("current_profile").get_ref<std::string&>();
    ImGui::TextUnformatted("Current");
    ImGui::SameLine();
    if (ImGui::BeginCombo("##combo", current_profile_name.c_str())) {
        for (const auto& profile : profiles) {
            const auto& name = profile.at("name").get_ref<const std::string&>();
            bool selected = name == current_profile_name;

            if (ImGui::Selectable(name.c_str(), selected)) {
                current_profile_name = name;
                config_modified = true;
                this->profile.reset();
                profile_dirty = false;
            }

            if (selected) ImGui::SetItemDefaultFocus();
        }

        ImGui::EndCombo();
    }

    if (ImGui::Button("Add Profile")) {
        ImGui::OpenPopup("##add_profile_popup");
    }

    if (ImGui::BeginPopup("##add_profile_popup")) {
        ImGui::SameLine();
        ImGui::InputText("Profile Name", &add_profile_name);
        if (ImGui::Button("Add")) {
            ImGui::CloseCurrentPopup();

            if (!add_profile_name.empty()) {
                scheduled_add = true;
                profile_error.clear();
            } else {
                profile_error = "Add error: profile name must not be empty.";
            }
        }

        ImGui::SameLine();

        if (ImGui::Button("Cancel")) {
            ImGui::CloseCurrentPopup();
            add_profile_name.clear();
        }

        ImGui::EndPopup();
    }

    ImGui::SameLine();

    if (ImGui::Button("Remove Profile")) {
        if (profiles.size() > 1) {
            scheduled_remove.emplace(current_profile_name);
            profile_error.clear();
        } else {
            profile_error = "Remove error: cannot remove all profiles.";
        }
    }

    if (!profile_error.empty()) {
        drawWarningText(profile_error.c_str());
    }

    ImGui::Separator();

    ImGui::TextColored(header_color, "Misc");
    ImGui::Spacing();

    if (ImGui::Button("Reload Config")) {
        scheduled_reload = true;
    }

    ImGui::SameLine();

    if (ImGui::Button("Update Schema URL")) {
        config["$schema"] =
            fmt::format("{}{}{}", state.context.usercontent_url, '/', state.context.schema_rel_path_str);
        config_modified = true;
    }

    ImGui::Separator();

    ImGui::TextColored(header_color, "Settings");
    ImGui::Spacing();

    bool save_profile = ImGui::Button("Save Changes");
    ImGui::SameLine();
    if (ImGui::Button("Discard Changes")) {
        profile.reset();
        profile_dirty = false;
    }
    ImGui::Spacing();

    const auto& profile_schema =
        state.config.schema.at("properties").at("profiles").at("items").at("properties").at("config");

    // TODO: Don't crash if profile is not in list.
    auto& profile_data = [this]() -> json& {
        if (!profile.has_value()) {
            profile.emplace(state.config.getCurrentProfile());
        }

        return profile.value();
    }();

    drawJsonObject(profile_schema, profile_data, std::nullopt);

    if (save_profile) {
        // TODO: Extract to member function
        bool found = false;
        for (auto& [_, value] : config.at("profiles").items()) {
            if (value.at("name").get_ref<const std::string&>() == current_profile_name) {
                value.at("config") = profile_data;
                found = true;
                break;
            }
        }
        if (!found) {
            throw std::runtime_error(fmt::format("profile not found: \"{}\"", current_profile_name));
        }
        profile_dirty = false;
        config_modified = true;
    }

    if (scheduled_add && this->profile.has_value()) {
        auto it = std::find_if(profiles.begin(), profiles.end(), [this](const json& p) {
            return p.at("name").get_ref<const std::string&>() == add_profile_name;
        });

        auto& profile = (it != profiles.end()) ? *it : profiles.emplace_back(json::object());
        profile["name"] = add_profile_name;
        profile["config"] = *this->profile;

        auto& current_profile = config.at("current_profile").get_ref<std::string&>();

        current_profile = add_profile_name;
        this->profile.reset();
        profile_dirty = false;

        add_profile_name.clear();
        scheduled_add = false;
        config_modified = true;
    }

    if (scheduled_remove.has_value()) {
        auto it = std::find_if(profiles.begin(), profiles.end(), [this](const json& p) {
            return p.at("name").get_ref<const std::string&>() == *scheduled_remove;
        });

        if (it != profiles.end()) {
            profiles.erase(it);
        }

        auto& current_profile = config.at("current_profile").get_ref<std::string&>();
        if (*scheduled_remove == current_profile) {
            current_profile = profiles.at(0).at("name");
            this->profile.reset();
            profile_dirty = false;
        }

        scheduled_remove.reset();
        config_modified = true;
    }

    if (config_modified) {
        state.config.save();
        state.feeder_needs_reload.store(true, std::memory_order_release);
    }

    // TODO: Workaround for scroll bar being partially off screen. Properly fix.
    ImGui::TextUnformatted("\n");

    ImGui::End();
}

[[nodiscard]] static auto defaultNotchPoints() {
    constexpr auto pi = std::numbers::pi;

    std::array<std::array<std::uint8_t, 2>, 8> points;
    for (std::size_t i = 0; i < points.size(); i++) {
        double angle = pi / 2 - (i * pi / 4);
        std::uint8_t x = util::lossy_cast<std::uint8_t>(127.0f * std::cos(angle) + 128.0f);
        std::uint8_t y = util::lossy_cast<std::uint8_t>(127.0f * std::sin(angle) + 128.0f);
        points[i] = {x, y};
    }
    return points;
}

[[nodiscard]] static Config::json defaultCalibration() {
    static constexpr auto defaultStickCenter = std::to_array<std::uint8_t>({128, 128});

    Config::json obj;
    auto& main_stick = obj["main_stick"];
    main_stick["notch_points"] = defaultNotchPoints();
    main_stick["stick_center"] = defaultStickCenter;
    obj["c_stick"] = main_stick;
    return obj;
}

static constexpr std::string_view defaultInversionMapping = "oot-vc";

static std::string getDescription(const json& value) {
    if (auto it = value.find("description"); it != value.end()) {
        return it->get_ref<const std::string&>();
    } else {
        return std::string{};
    }
}

static void drawDescription(const json& value) {
    if (!getDescription(value).empty()) {
        ImGui::SameLine();
        ImGui::TextDisabled("(?)");
        if (ImGui::IsItemHovered()) {
            ImGui::BeginTooltip();
            ImGui::PushTextWrapPos(ImGui::GetFontSize() * 35.0f);
            ImGui::TextUnformatted(getDescription(value).c_str());
            ImGui::PopTextWrapPos();
            ImGui::EndTooltip();
        }
    }
}

void ConfigEditor::drawJsonObject(const json& schema_obj, json& data_obj,
                                  std::optional<std::reference_wrapper<const std::string>> name, bool is_top_level) {
    if (auto it = schema_obj.find("type"); it != schema_obj.end()) {
        const auto& type = it->get_ref<const std::string&>();
        if (type == "object") {
            bool draw_properties = true;
            if (name.has_value()) {
                if (is_top_level) {
                    if (!ImGui::CollapsingHeader(name->get().c_str(), ImGuiTreeNodeFlags_DefaultOpen)) {
                        draw_properties = false;
                    }
                } else {
                    if (!ImGui::TreeNodeEx(name->get().c_str(), ImGuiTreeNodeFlags_DefaultOpen)) {
                        draw_properties = false;
                    }
                }
            }

            if (draw_properties) {
                const auto& properties = schema_obj.at("properties");
                for (const auto& [child_name, child_schema_obj] : properties.items()) {
                    drawJsonObject(child_schema_obj, data_obj.at(child_name), child_name, !name.has_value());
                }
            }

            if (name.has_value() && !is_top_level && draw_properties) ImGui::TreePop();
        } else if (type == "boolean") {
            if (ImGui::Checkbox(name->get().c_str(), data_obj.get_ptr<bool*>())) {
                profile_dirty = true;
            }
            drawDescription(schema_obj);
        } else if (type == "integer") {
            auto& field = data_obj.get_ref<json::number_integer_t&>();
            const auto minimum = schema_obj.find("minimum");
            const auto maximum = schema_obj.find("maximum");

            if (maximum != schema_obj.end()) {
                float width = (std::floorf(std::log10f(maximum->get<json::number_integer_t>())) + 2.0f) *
                              ImGui::CalcTextSize("0").x;
                ImGui::SetNextItemWidth(width);
            }

            // TODO: Complain if field is too large.
            int v = util::lossy_cast<int>(field);
            if (ImGui::InputInt(name->get().c_str(), &v, 0)) {
                field = static_cast<json::number_integer_t>(v);
                if (minimum != schema_obj.end()) {
                    field = std::max(field, minimum->get<json::number_integer_t>());
                }
                if (maximum != schema_obj.end()) {
                    field = std::min(field, maximum->get<json::number_integer_t>());
                }

                profile_dirty = true;
            }
            drawDescription(schema_obj);
        } else if (type == "number") {
            auto& field = data_obj.get_ref<json::number_float_t&>();
            const auto minimum = schema_obj.find("minimum");
            const auto maximum = schema_obj.find("maximum");

            double v = util::lossy_cast<double>(field);
            if (ImGui::InputDouble(name->get().c_str(), &v, 0.1, 0.0, "%.2f")) {
                field = static_cast<json::number_float_t>(v);
                if (minimum != schema_obj.end()) {
                    field = std::max(field, minimum->get<json::number_float_t>());
                }
                if (maximum != schema_obj.end()) {
                    field = std::min(field, maximum->get<json::number_float_t>());
                }
                profile_dirty = true;
            }
            drawDescription(schema_obj);
        } else if (type == "string") {
            const auto variants = schema_obj.find("enum");
            if (variants == schema_obj.end()) {
                drawWarningText(fmt::format("{}: non-enum strings unsupported", name->get()).c_str());
                return;
            }

            auto& current_variant = data_obj.get_ref<std::string&>();
            if (ImGui::BeginCombo(name->get().c_str(), current_variant.c_str())) {
                for (const auto& [_, variant_obj] : variants->items()) {
                    const auto& variant = variant_obj.get_ref<const std::string&>();
                    bool selected = variant == current_variant;

                    if (ImGui::Selectable(variant.c_str(), selected)) {
                        current_variant = variant;
                        profile_dirty = true;
                    }

                    if (selected) ImGui::SetItemDefaultFocus();
                }

                ImGui::EndCombo();
            }
            drawDescription(schema_obj);
        } else if (type == "array") {
            std::size_t depth = 0;
            const auto* value_type_obj = &schema_obj;
            while (true) {
                if (auto it = value_type_obj->find("items"); it != value_type_obj->end()) {
                    depth++;
                    value_type_obj = &*it;
                } else {
                    break;
                }
            }

            auto& data_array = data_obj.get_ref<json::array_t&>();

            if (data_array.empty()) {
                drawWarningText(fmt::format("Empty array: {}", name->get()).c_str());
                return;
            } else if (depth > 2) {
                drawWarningText(fmt::format("Array depth greater than 2 not supported: {}", name->get()).c_str());
                return;
            }

            ImGui::TextUnformatted(name->get().c_str());

            // TODO: Figure out a sane height computation.
            const float height = depth * 50.0f + 4.0f;
            if (!ImGui::BeginTable(name->get().c_str(), static_cast<int>(data_array.size()),
                                   ImGuiTableFlags_NoSavedSettings | ImGuiTableFlags_Borders | ImGuiTableFlags_ScrollX,
                                   ImVec2(0.0f, height))) {
                return;
            }

            for (std::size_t col = 0; col < data_array.size(); col++) {
                ImGui::TableSetupColumn(fmt::format("{}", col).c_str(), ImGuiTableColumnFlags_WidthFixed);
            }

            ImGui::TableHeadersRow();

            switch (depth) {
                case 1:
                    ImGui::TableNextRow();
                    for (std::size_t col = 0; col < data_array.size(); col++) {
                        ImGui::TableSetColumnIndex(static_cast<int>(col));
                        const auto child_name = fmt::format("##{}[{}]", name->get(), col);
                        drawJsonObject(*value_type_obj, data_array.at(col), child_name, false);
                    }
                    break;
                case 2: {
                    std::size_t rows = 0;
                    for (const auto& row_data : data_array) {
                        if (row_data.size() > rows) rows = row_data.size();
                    }

                    for (std::size_t row = 0; row < rows; row++) {
                        ImGui::TableNextRow();

                        for (std::size_t col = 0; col < data_array.size(); col++) {
                            ImGui::TableSetColumnIndex(static_cast<int>(col));
                            const auto child_name = fmt::format("##{}[{}][{}]", name->get(), col, row);
                            auto& elem = data_array.at(col).get_ref<json::array_t&>().at(row);
                            drawJsonObject(*value_type_obj, elem, child_name, false);
                        }
                    }
                    break;
                }
                default:
                    throw std::runtime_error{fmt::format("Unexpected array depth: {}", depth)};
            }

            ImGui::EndTable();
        } else {
            drawWarningText(fmt::format("Unsupported type: {}", type).c_str());
        }
    } else if (auto it = schema_obj.find("anyOf"); it != schema_obj.end()) {
        if (!name.has_value()) {
            drawWarningText("Unnamed anyOf");
            return;
        } else if (!ImGui::TreeNodeEx(name->get().c_str(), ImGuiTreeNodeFlags_DefaultOpen)) {
            return;
        }

        drawDescription(schema_obj);

        auto non_null_variant = std::find_if(it->begin(), it->end(), [](const json& e) {
            auto type = e.find("type");
            return type != e.end() && type->get_ref<const std::string&>() != "null";
        });

        bool present = !data_obj.is_null();
        if (ImGui::Checkbox("Present", &present)) {
            // TODO: Safer check, ensure object hierarchy is correct
            if (data_obj.is_null()) {
                if (name->get() == "data") {
                    data_obj = defaultCalibration();
                } else if (name->get() == "inversion_mapping") {
                    data_obj = defaultInversionMapping;
                } else {
                    throw std::runtime_error{fmt::format("Type for key has no default: {}\n", name->get())};
                }
            } else {
                data_obj = nullptr;
            }
            profile_dirty = true;
        }

        if (present) {
            ImGui::Separator();
            static const std::string value_str = "Value";
            drawJsonObject(*non_null_variant, data_obj, value_str, false);
        }

        ImGui::TreePop();
    } else {
        drawWarningText("Object without \"type\" or \"anyOf\" unsupported");
    }
}
