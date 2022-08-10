#include "calibration_window.h"

#include <fmt/core.h>

void CalibrationWindow::updateInputs(Inputs inputs) {
    std::scoped_lock lock(mutex);
    this->inputs = inputs;
}

void CalibrationWindow::drawAndUpdate(const char* title, bool& open) {
    if (!open) {
        return;
    }

    if (!ImGui::Begin(title, &open, ImGuiWindowFlags_NoFocusOnAppearing)) {
        ImGui::End();
        return;
    }

    if (ImGui::Button("Calibrate")) {
        ImGui::OpenPopup("##calibration_popup");
        is_calibrating.store(true, std::memory_order_release);
    }

    if (ImGui::BeginPopup("##calibration_popup")) {
        ImGui::SameLine();
        ImGui::TextUnformatted("Calibrating...");

        if (ImGui::Button("Cancel")) {
            ImGui::CloseCurrentPopup();
            is_calibrating.store(false, std::memory_order_release);
        }

        ImGui::EndPopup();
    } else {
        is_calibrating.store(false, std::memory_order_release);
    }

    const auto inputs = [this]() {
        std::scoped_lock lock(this->mutex);
        return this->inputs;
    }();

    if (inputs.active_stages & STAGE_RAW) {
        ImGui::TextUnformatted(fmt::format("Raw: ({}, {}) | ({}, {})", inputs.main_stick.raw.x, inputs.main_stick.raw.y,
                                           inputs.c_stick.raw.x, inputs.c_stick.raw.y)
                                   .c_str());
    }

    if (inputs.active_stages & STAGE_MAPPED) {
        ImGui::TextUnformatted(fmt::format("Mapped: ({}, {}) | ({}, {})", inputs.main_stick.mapped.x,
                                           inputs.main_stick.mapped.y, inputs.c_stick.mapped.x, inputs.c_stick.mapped.y)
                                   .c_str());
    }

    if (inputs.active_stages & STAGE_CALIBRATED) {
        ImGui::TextUnformatted(fmt::format("Calibrated: ({}, {}) | ({}, {})", inputs.main_stick.calibrated.x,
                                           inputs.main_stick.calibrated.y, inputs.c_stick.calibrated.x,
                                           inputs.c_stick.calibrated.y)
                                   .c_str());
    }

    if (inputs.active_stages & STAGE_RAW) {
        ImGui::TextUnformatted(fmt::format("A Pressed: {}", inputs.a_pressed != 0).c_str());
    }

    ImGui::End();
}
