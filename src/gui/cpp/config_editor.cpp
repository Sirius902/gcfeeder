#include "config_editor.h"

#include <fmt/core.h>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <exception>
#include <numbers>
#include <ranges>
#include <utility>

#include "util.h"

namespace ranges = std::ranges;

using json = Config::json;

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

    bool config_modified = false;

    ImGui::TextColored(header_color, "Profiles");
    ImGui::Spacing();

    auto& profiles = config.at("profiles");

    auto& current_profile_name = config.at("current_profile").get_ref<std::string&>();
    ImGui::TextUnformatted("Current");
    ImGui::SameLine();
    if (ImGui::BeginCombo("##combo", current_profile_name.c_str())) {
        for (const auto& [_, profile] : profiles.items()) {
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

    const auto& profile_properties =
        state.config.schema.at("properties").at("profiles").at("items").at("properties").at("config").at("properties");

    auto& current_profile = [&]() -> json& {
        if (!profile.has_value()) {
            for (auto& [_, value] : config.at("profiles").items()) {
                if (value.at("name").get_ref<const std::string&>() == current_profile_name) {
                    profile = value.at("config");
                    break;
                }
            }

            if (!profile.has_value()) {
                // TODO: Don't crash if profile is not in list.
                throw std::runtime_error(fmt::format("profile not found: \"{}\"", current_profile_name));
            }
        }

        return profile.value();
    }();

    drawObject(profile_properties, current_profile);

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
        profile_dirty = false;
        config_modified = true;
    }

    if (config_modified) {
        state.config.load();
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

void ConfigEditor::drawObject(const json& properties, json& data) {
    for (const auto& [key, value] : properties.items()) {
        const auto getDescription = [](const json& value) {
            if (auto it = value.find("description"); it != value.end()) {
                return it->get_ref<const std::string&>();
            } else {
                return std::string{};
            }
        };

        static const auto addDescription = [&](const json& value) {
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
        };

        static const auto warningText = [](const char* text) {
            constexpr auto warn_color = ImVec4(0.9f, 0.2f, 0.2f, 1.0f);
            ImGui::PushStyleColor(ImGuiCol_Text, warn_color);
            ImGui::TextUnformatted(text);
            ImGui::PopStyleColor();
        };

        static const auto drawTypedObject = [this](const json& schema_obj, const std::string& key, const json& value,
                                                   json& data) {
            const auto& type = schema_obj.get_ref<const std::string&>();

            if (type == "object") {
                ImGui::SetNextItemOpen(true);
                if (ImGui::TreeNode(key.c_str())) {
                    addDescription(value);
                    drawObject(value.at("properties"), data.at(key));
                    ImGui::TreePop();
                } else {
                    addDescription(value);
                }
            } else if (type == "boolean") {
                if (ImGui::Checkbox(key.c_str(), data.at(key).get_ptr<bool*>())) {
                    profile_dirty = true;
                }
                addDescription(value);
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

                    profile_dirty = true;
                }
                addDescription(value);
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
                    profile_dirty = true;
                }
                addDescription(value);
            } else if (type == "string") {
                const auto variants = value.find("enum");
                if (variants == value.end()) {
                    warningText(fmt::format("{}: non-enum strings unsupported", key).c_str());
                    return;
                }

                auto& current_variant = data.at(key).get_ref<std::string&>();
                if (ImGui::BeginCombo(key.c_str(), current_variant.c_str())) {
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
                addDescription(value);
            } else if (type == "array") {
                const auto& value_type = value.at("items").at("type").get_ref<const std::string&>();
                const auto min_items_it = value.find("minItems");

                if (min_items_it != value.end() && value_type == "integer") {
                    auto& arr = data.at(key).get_ref<json::array_t&>();
                    const auto min_items = min_items_it->get<json::number_unsigned_t>();
                    for (json::number_unsigned_t i = 0; i < min_items; i++) {
                        if (i > 0) ImGui::SameLine();

                        auto& elem = arr.at(i).get_ref<json::number_integer_t&>();
                        int v = util::lossy_cast<int>(elem);
                        if (ImGui::InputInt(fmt::format("##array[{}]", i).c_str(), &v, 0)) {
                            elem = util::lossy_cast<json::number_integer_t>(v);
                            profile_dirty = true;
                        }
                    }
                } else {
                    warningText(fmt::format("non-integer arrays without minItems unsupported: {}", key).c_str());
                }
            } else {
                const auto warning = fmt::format("Unimplemented json type: {}\n", type);
                ImGui::TextUnformatted(key.c_str());
                addDescription(value);
                warningText(warning.c_str());
            }
        };

        if (auto it = value.find("anyOf"); it != value.end()) {
            bool is_nullable = [&it]() {
                for (const auto& [key, value] : it->items()) {
                    if (auto type = value.find("type"); type != value.end()) {
                        if (type->get_ref<const std::string&>() == "null") {
                            return it->size() == 2;
                        }
                    }
                }
                return false;
            }();

            auto non_null_variant = std::find_if(it->begin(), it->end(), [](const auto& e) -> bool {
                auto type = e.find("type");
                return type != e.end() && type->template get_ref<const std::string&>() != "null";
            });

            if (is_nullable && non_null_variant != it->end()) {
                auto& nullable = data.at(key.c_str());
                bool checked = !nullable.is_null();
                if (ImGui::Checkbox("##check", &checked)) {
                    // TODO: Safer check, ensure object hierarchy is correct
                    if (nullable.is_null()) {
                        if (key == "data") {
                            nullable = defaultCalibration();
                        } else if (key == "inversion_mapping") {
                            nullable = defaultInversionMapping;
                        } else {
                            fmt::print(stderr, "Type for key has no default: {}\n", key);
                        }
                    } else {
                        nullable = nullptr;
                    }
                    profile_dirty = true;
                }
                ImGui::SameLine();
                ImGui::TextUnformatted(key.c_str());
                addDescription(value);
                if (checked) {
                    drawTypedObject(non_null_variant->at("type"), key, *non_null_variant, data);
                }
            } else {
                ImGui::TextUnformatted(key.c_str());
                warningText(fmt::format("anyOf without null / two variants not supported").c_str());
            }
        } else if (auto it = value.find("type"); it != value.end()) {
            drawTypedObject(*it, key, value, data);
        } else {
            const auto warning = fmt::format("Unknown json property with name: {}\n", key);
            ImGui::TextUnformatted(key.c_str());
            addDescription(value);
            warningText(warning.c_str());
        }
    }
}
