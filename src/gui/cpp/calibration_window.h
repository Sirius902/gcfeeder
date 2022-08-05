#pragma once

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

class CalibrationWindow {
public:
    void drawAndUpdate(const char* title, bool& open);
};
