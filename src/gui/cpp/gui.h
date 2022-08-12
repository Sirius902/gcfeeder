#pragma once

#include <atomic>
#include <utility>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>
#include <imgui_internal.h>

#include "calibration_window.h"
#include "config_editor.h"
#include "gui_main.h"
#include "gui_state.h"

class Gui {
public:
    Gui(UIContext& context, AppLog& log)
        : state{context, log}, config_editor(state), calibration_window(state, getCalibrationApplyCallback()) {}

    bool isFeederReloadNeeded() const { return state.feeder_needs_reload.load(std::memory_order_acquire); }
    void notifyFeederReload() { state.feeder_needs_reload.store(false, std::memory_order_release); }

    void updateInputs(Inputs inputs) { calibration_window.updateInputs(inputs); }
    bool isCalibrating() const { return calibration_window.isCalibrating(); }

    void drawAndUpdate();

private:
    GuiState state;
    ConfigEditor config_editor;
    CalibrationWindow calibration_window;

    bool draw_config_editor = true;
    bool draw_calibration_window = true;
    bool draw_log = true;
    ImGuiID dockspace_id;

    bool draw_demo_window = false;

    CalibrationWindow::ApplyCallback getCalibrationApplyCallback() {
        return [this](Config::json&& calibration) { config_editor.updateProfileCalibration(std::move(calibration)); };
    }

    void drawCalibrationData(const char* title, bool& open);
};
