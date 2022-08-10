#pragma once

#include <atomic>
#include <mutex>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

#include "gui_main.h"

class CalibrationWindow {
private:
    std::mutex mutex;
    Inputs inputs{};
    std::atomic<bool> is_calibrating{false};

public:
    bool isCalibrating() const { return is_calibrating.load(std::memory_order_acquire); }

    void updateInputs(Inputs inputs);
    void drawAndUpdate(const char* title, bool& open);
};
