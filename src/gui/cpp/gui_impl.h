#pragma once

#include "app_log.h"
#include "gui.h"

class Gui {
private:
    AppLog log;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    bool rumble_enabled = true;

    bool draw_demo_window = false;

public:
    AppLog& getLog() { return log; }

    void drawAndUpdate(UIContext& context);
};
