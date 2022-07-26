#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>

#include "app_log.h"
#include "gui_main.h"

class Gui {
private:
    AppLog log;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    bool rumble_enabled = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

public:
    AppLog& getLog() { return log; }

    void drawAndUpdate(UIContext& context);
};
