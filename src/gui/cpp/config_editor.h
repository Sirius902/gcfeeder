#pragma once

#include <concepts>
#include <functional>
#include <optional>
#include <string>
#include <utility>

#include "config.h"
#include "gui_state.h"

class ConfigEditor {
private:
    GuiState& state;
    std::optional<Config::json> profile;
    std::string add_profile_name;
    std::string profile_error;

    bool profile_dirty = false;
    bool scheduled_reload = false;
    bool scheduled_add = false;
    std::optional<std::string> scheduled_remove;

    void drawJsonObject(const Config::json& schema_obj, Config::json& data_obj,
                        std::optional<std::reference_wrapper<const std::string>> name, bool is_top_level = true);

public:
    ConfigEditor(GuiState& state) : state(state) {}

    template <std::convertible_to<Config::json> Json>
    void updateProfileStickCalibration(Json&& calibration) {
        if (profile.has_value()) {
            auto& config_calibration = profile->at("calibration");
            config_calibration.at("enabled") = true;
            config_calibration.at("stick_data") = std::forward<Json>(calibration);
            profile_dirty = true;
        }
    }

    template <std::convertible_to<Config::json> Json>
    void updateProfileTriggerCalibration(Json&& calibration) {
        if (profile.has_value()) {
            auto& config_calibration = profile->at("calibration");
            config_calibration.at("enabled") = true;
            config_calibration.at("trigger_data") = std::forward<Json>(calibration);
            profile_dirty = true;
        }
    }

    void drawAndUpdate(const char* title, bool& open);
};
