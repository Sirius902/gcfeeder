#pragma once

#include "config.h"
#include "gui_state.h"

class ConfigEditor {
private:
    GuiState& state;
    std::optional<Config::json> profile;

    bool profile_dirty = false;
    bool scheduled_reload = false;

    void drawObject(const Config::json& properties, Config::json& data);

public:
    ConfigEditor(GuiState& state) : state(state) {}

    void drawAndUpdate(const char* title, bool& open);
};
