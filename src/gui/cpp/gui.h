#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>

#include <atomic>
#include <concepts>
#include <nlohmann/json.hpp>
#include <optional>
#include <utility>

#include "app_log.h"
#include "gui_main.h"

class Gui {
private:
    using json = nlohmann::ordered_json;

    UIContext& context;
    AppLog& log;
    json config_schema;
    std::optional<json> config;
    std::optional<json> editor_profile;
    bool editor_profile_dirty = false;
    bool scheduled_reload = false;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

    void drawConfigEditor(const char* title, bool& open);
    void drawConfigEditorObject(const json& properties, json& data);
    void drawCalibrationData(const char* title, bool& open);

    void loadConfig();
    void saveConfig();

public:
    std::atomic_bool feeder_needs_reload{true};

    template <std::convertible_to<json> T>
    Gui(UIContext& context, AppLog& log, T&& config_schema)
        : context(context), log(log), config_schema(std::forward<T>(config_schema)) {}

    void drawAndUpdate();
};
