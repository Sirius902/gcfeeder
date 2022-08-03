#pragma once

#include <atomic>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <fmt/core.h>
#include <imgui.h>
#include <imgui_internal.h>

#include "config_editor.h"
#include "gui_main.h"
#include "gui_state.h"

class Gui {
public:
    Gui(UIContext& context, AppLog& log) : state{context, log}, config_editor(state) {}

    bool isFeederReloadNeeded() const { return state.feeder_needs_reload.load(std::memory_order_acquire); }
    void notifyFeederReload() { state.feeder_needs_reload.store(false, std::memory_order_release); }

    void drawAndUpdate();

private:
    GuiState state;
    ConfigEditor config_editor;

    bool draw_config = true;
    bool draw_calibration_data = true;
    bool draw_log = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

    void drawCalibrationData(const char* title, bool& open);
};
