#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>

#include <nlohmann/json.hpp>
#include <utility>

#include "app_log.h"
#include "gui_main.h"

class Gui {
private:
    nlohmann::json config_schema;
    AppLog& log;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    bool rumble_enabled = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

public:
    template <typename T>
    Gui(AppLog& log, T&& config_schema) : config_schema(std::forward<T>(config_schema)), log(log) {}

    AppLog& getLog() { return log; }

    void drawAndUpdate(UIContext& context);
};
