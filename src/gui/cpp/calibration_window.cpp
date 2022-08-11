#include "calibration_window.h"

#include <fmt/core.h>

#include <string_view>
#include <utility>

#include "util.h"

namespace chrono = std::chrono;

void CalibrationWindow::updateInputs(Inputs inputs) {
    std::scoped_lock lock(mutex);
    this->inputs = inputs;
}

static constexpr auto notch_names = std::to_array<std::string_view>({
    "top",
    "top-right",
    "right",
    "bottom-right",
    "bottom",
    "bottom-left",
    "left",
    "top-left",
});

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

    const auto inputs = [this]() {
        std::scoped_lock lock(this->mutex);
        return this->inputs;
    }();

    drawPopup(inputs);

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

void CalibrationWindow::applyCalibration() {
    Config::json calibration;

    auto& main_stick = calibration["main_stick"];
    main_stick["notch_points"] = main_stick_points;
    main_stick["stick_center"] = *main_stick_center;

    auto& c_stick = calibration["c_stick"];
    c_stick["notch_points"] = c_stick_points;
    c_stick["stick_center"] = *c_stick_center;

    apply_fn(std::move(calibration));
}

void CalibrationWindow::drawPopup(const Inputs& inputs) {
    static auto endCalibration = [this]() {
        is_calibrating.store(false, std::memory_order_release);
        main_stick_center.reset();
        c_stick_center.reset();
        main_stick_points.clear();
        c_stick_points.clear();
    };

    static auto shouldConfirm = [&]() -> bool {
        if (inputs.a_pressed && !was_pressed) {
            was_pressed = true;
            return true;
        } else if (!inputs.a_pressed) {
            was_pressed = false;
        }

        return false;
    };

    static auto drawCalibrationProgress = [&]() {
        bool is_main_stick = true;
        while (true) {
            std::string_view stick_name = is_main_stick ? "main " : "C-";
            auto& center = is_main_stick ? main_stick_center : c_stick_center;
            auto& points = is_main_stick ? main_stick_points : c_stick_points;
            const auto& stick = is_main_stick ? inputs.main_stick : inputs.c_stick;

            if (center.has_value()) {
                ImGui::TextUnformatted(
                    fmt::format("{}stick center: ({}, {})", stick_name, (*center)[0], (*center)[1]).c_str());
            } else {
                ImGui::TextUnformatted(fmt::format("Center {}stick and press A", stick_name).c_str());
                if (shouldConfirm()) {
                    center = {stick.raw.x, stick.raw.y};
                }
                return;
            }

            for (std::size_t notch = 0; notch < notch_names.size(); notch++) {
                const auto& notch_name = notch_names.at(notch);

                if (notch < points.size()) {
                    const auto& point = points.at(notch);
                    ImGui::TextUnformatted(
                        fmt::format("{}stick {} notch: ({}, {})", stick_name, notch_name, point[0], point[1]).c_str());
                } else {
                    ImGui::TextUnformatted(
                        fmt::format("Move {}stick to center then to {} then press A", stick_name, notch_name).c_str());
                    if (shouldConfirm()) {
                        points.push_back({stick.raw.x, stick.raw.y});
                    }
                    return;
                }
            }

            if (is_main_stick) {
                is_main_stick = false;
            } else {
                break;
            }
        }
    };

    static auto isCalibrationFinished = [this]() { return c_stick_points.size() >= notch_names.size(); };

    if (ImGui::BeginPopup("##calibration_popup")) {
        if (ImGui::Button("Cancel")) {
            ImGui::CloseCurrentPopup();
            endCalibration();
        }

        ImGui::Separator();

        ImGui::BeginChild("scrolling", ImVec2(500.0f, 300.0f), false, ImGuiWindowFlags_HorizontalScrollbar);

        ImGui::TextUnformatted("Calibrating...");
        drawCalibrationProgress();

        if (isCalibrationFinished()) {
            ImGui::Separator();

            ImGui::TextUnformatted("Calibration finished. Apply to config editor profile?");
            if (ImGui::Button("Apply")) {
                applyCalibration();
                ImGui::CloseCurrentPopup();
                endCalibration();
            }

            ImGui::SameLine();

            if (ImGui::Button("Discard")) {
                ImGui::CloseCurrentPopup();
                endCalibration();
            }
        }

        if (ImGui::GetScrollY() >= ImGui::GetScrollMaxY()) {
            ImGui::SetScrollHereY(1.0f);
        }

        ImGui::EndChild();
        ImGui::EndPopup();
    } else {
        if (isCalibrating()) endCalibration();
    }
}
