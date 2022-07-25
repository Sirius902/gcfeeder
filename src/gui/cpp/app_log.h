#pragma once

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

#include <mutex>

class AppLog {
private:
    ImGuiTextBuffer buffer;
    ImGuiTextFilter filter;
    ImVector<int> line_offsets;
    bool auto_scroll;
    std::mutex mutex;

public:
    AppLog();

    void clear();
    void add(const char* message);
    void draw(const char* title, bool& open);
};
