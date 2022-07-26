#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>

#include <nlohmann/json.hpp>
#include <utility>

#include "app_log.h"
#include "gui_main.h"

class Gui {
private:
    UIContext& context;
    AppLog& log;
    nlohmann::json config;
    nlohmann::json config_schema;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

    void drawConfigEditor(const char* title, bool& open);
    void drawCalibrationData(const char* title, bool& open);

    void loadConfig();
    void saveConfig();

public:
    template <typename T>
    Gui(UIContext& context, AppLog& log, T&& config_schema)
        : context(context), log(log), config_schema(std::forward<T>(config_schema)) {
        loadConfig();
    }

    AppLog& getLog() { return log; }

    void drawAndUpdate();
};
