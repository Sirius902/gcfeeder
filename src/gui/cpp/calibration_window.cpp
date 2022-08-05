#include "calibration_window.h"

void CalibrationWindow::drawAndUpdate(const char* title, bool& open) {
    if (!open) {
        return;
    }

    if (!ImGui::Begin(title, &open, ImGuiWindowFlags_NoFocusOnAppearing)) {
        ImGui::End();
        return;
    }

    ImGui::End();
}
