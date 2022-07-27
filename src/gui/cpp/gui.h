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
    UIContext& context;
    AppLog& log;
    std::optional<nlohmann::json> config;
    nlohmann::json config_schema;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

    void drawConfigEditor(const char* title, bool& open);
    void drawConfigEditorObject(const nlohmann::json& properties, nlohmann::json& data);
    void drawCalibrationData(const char* title, bool& open);

    void loadConfig();
    void saveConfig();

public:
    std::atomic_bool feeder_needs_reload{true};

    template <std::convertible_to<nlohmann::json> T>
    Gui(UIContext& context, AppLog& log, T&& config_schema)
        : context(context), log(log), config_schema(std::forward<T>(config_schema)) {}

    AppLog& getLog() { return log; }

    void drawAndUpdate();
};
