#pragma once

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui/imgui.h>

struct AppLog {
    ImGuiTextBuffer buffer;
    ImGuiTextFilter filter;
    ImVector<int> line_offsets;
    bool auto_scroll;

    AppLog();

    void clear();
    void add(const char* message);
    void draw(const char* title, bool& open);
};
