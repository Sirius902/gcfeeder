#pragma once

#include <concepts>
#include <functional>
#include <optional>
#include <string>

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
    void updateProfileCalibration(Json&& calibration) {
        if (profile.has_value()) {
            profile->at("calibration").at("data") = calibration;
            profile_dirty = true;
        }
    }

    void drawAndUpdate(const char* title, bool& open);
};
