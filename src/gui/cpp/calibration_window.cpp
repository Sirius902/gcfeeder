#include "calibration_window.h"

#include <fmt/core.h>

#include <cmath>
#include <cstddef>
#include <string_view>
#include <utility>

#include "util.h"

namespace chrono = std::chrono;

using json = Config::json;

struct StickCalibration {
    using Point = CalibrationWindow::Point;

    std::array<Point, 8> notch_points;
    Point stick_center;

    StickCalibration() = default;

    StickCalibration(const json& data) {
        const auto& notch_points_data = data.at("notch_points").get_ref<const json::array_t&>();
        const auto& stick_center_data = data.at("stick_center").get_ref<const json::array_t&>();

        for (std::size_t i = 0; i < notch_points_data.size(); i++) {
            const auto& point = notch_points_data.at(i).get_ref<const json::array_t&>();

            for (std::size_t j = 0; j < point.size(); j++) {
                const auto value = util::lossy_cast<std::uint8_t>(point.at(j).get<json::number_unsigned_t>());
                notch_points[i][j] = value;
            }
        }

        for (std::size_t i = 0; i < stick_center_data.size(); i++) {
            const auto value = util::lossy_cast<std::uint8_t>(stick_center_data.at(i).get<json::number_unsigned_t>());
            stick_center[i] = value;
        }
    }
};

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

static const auto main_stick_color = ImColor(255, 255, 255);
static const auto c_stick_color = ImColor(255, 255, 0);

static CalibrationWindow::Point toPoint(Vec2 v) { return {v.x, v.y}; }

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

    const auto inputs = [this]() {
        std::scoped_lock lock(this->mutex);
        return this->inputs;
    }();

    if (!(inputs.active_stages & STAGE_RAW)) ImGui::BeginDisabled();

    if (ImGui::Button("Calibrate")) {
        ImGui::OpenPopup("##calibration_popup");
        is_calibrating.store(true, std::memory_order_release);
    }

    ImGui::Checkbox("View Calibration Points", &view_calibration_points);

    if (!(inputs.active_stages & STAGE_RAW)) ImGui::EndDisabled();

    drawPopup(inputs);

    StickCalibration main_stick_calibration;
    std::optional<std::span<Point>> main_stick_points;
    std::optional<Point> main_stick_center;

    StickCalibration c_stick_calibration;
    std::optional<std::span<Point>> c_stick_points;
    std::optional<Point> c_stick_center;

    if (state.config.isLoaded() && view_calibration_points) {
        const auto& profile = state.config.getCurrentProfile();
        const auto& calibration_data = profile.at("calibration").at("stick_data");

        if (!calibration_data.is_null()) {
            const auto& main_stick_data = calibration_data.at("main_stick");
            const auto& c_stick_data = calibration_data.at("c_stick");

            main_stick_calibration = StickCalibration(main_stick_data);
            main_stick_points = std::span(main_stick_calibration.notch_points);
            main_stick_center = main_stick_calibration.stick_center;

            c_stick_calibration = StickCalibration(c_stick_data);
            c_stick_points = std::span(c_stick_calibration.notch_points);
            c_stick_center = c_stick_calibration.stick_center;
        }
    }

    if (inputs.active_stages & STAGE_RAW) {
        ImGui::TextUnformatted("Raw");
        drawStick("main_raw", inputs.main_stick.raw, main_stick_color, main_stick_points, main_stick_center);
        ImGui::SameLine();
        drawStick("c_raw", inputs.c_stick.raw, c_stick_color, c_stick_points, c_stick_center);
    }

    if (inputs.active_stages & STAGE_MAPPED) {
        ImGui::TextUnformatted("Mapped");
        drawStick("main_mapped", inputs.main_stick.mapped, main_stick_color, main_stick_points, main_stick_center);
        ImGui::SameLine();
        drawStick("c_mapped", inputs.c_stick.mapped, c_stick_color, c_stick_points, c_stick_center);
    }

    if (inputs.active_stages & STAGE_CALIBRATED) {
        static const auto points = Config::defaultNotchPoints();
        const auto points_span = view_calibration_points
                                     ? std::optional{std::span(points.data(), points.data() + points.size())}
                                     : std::nullopt;
        const auto center = view_calibration_points ? std::optional{Config::default_stick_center} : std::nullopt;

        ImGui::TextUnformatted("Calibrated");
        drawStick("main_calibrated", inputs.main_stick.calibrated, main_stick_color, points_span, center);
        ImGui::SameLine();
        drawStick("c_calibrated", inputs.c_stick.calibrated, c_stick_color, points_span, center);
    }

    if (inputs.active_stages & STAGE_SCALED) {
        ImGui::TextUnformatted("Scaled");
        drawStick("main_scaled", inputs.main_stick.scaled, main_stick_color);
        ImGui::SameLine();
        drawStick("c_scaled", inputs.c_stick.scaled, c_stick_color);
    }

    ImGui::End();
}

void CalibrationWindow::applyStickCalibration() {
    json calibration;

    auto& main_stick = calibration["main_stick"];
    main_stick["notch_points"] = main_stick_points;
    main_stick["stick_center"] = *main_stick_center;

    auto& c_stick = calibration["c_stick"];
    c_stick["notch_points"] = c_stick_points;
    c_stick["stick_center"] = *c_stick_center;

    stick_apply_fn(std::move(calibration));
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
        if (inputs.a_pressed && !a_was_pressed) {
            a_was_pressed = true;
            return true;
        } else if (!inputs.a_pressed) {
            a_was_pressed = false;
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
            const auto& color = is_main_stick ? main_stick_color : c_stick_color;

            drawStick(fmt::format("{} stick", stick_name).c_str(), stick.raw, color, points, center);

            if (!center.has_value()) {
                ImGui::TextUnformatted(fmt::format("Center {}stick and press A", stick_name).c_str());
                if (shouldConfirm()) {
                    center = {stick.raw.x, stick.raw.y};
                }
                return;
            }

            for (std::size_t notch = 0; notch < notch_names.size(); notch++) {
                const auto& notch_name = notch_names.at(notch);

                if (notch >= points.size()) {
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
                applyStickCalibration();
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

static ImColor colorWithAlpha(ImColor color, float alpha) {
    color.Value.w = alpha;
    return color;
}

void CalibrationWindow::drawStick(const char* str_id, Vec2 stick_pos, ImColor main_color,
                                  std::optional<std::span<const Point>> points, std::optional<Point> center) {
    main_color.Value.w = 1.0f;
    static constexpr auto size = 60.0f;
    static constexpr auto octagon_radius = size * 0.4f;
    const auto background_color = ImColor(main_color.Value.x, main_color.Value.y, main_color.Value.z, 0.6f);
    const auto calibration_color =
        ImColor(1.0f - background_color.Value.x, 1.0f - background_color.Value.y, 1.0f - background_color.Value.z);

    ImGui::BeginChild(str_id, ImVec2(size, size));

    const auto cursor_pos = ImGui::GetCursorScreenPos();
    const auto center_pos = ImVec2(cursor_pos.x + 0.5f * size, cursor_pos.y + 0.5f * size);

    ImDrawList* draw_list = ImGui::GetWindowDrawList();

    const auto drawStickPoint = [&](Point coords, const ImColor& color) {
        static constexpr float half_size = size / 20.0f;

        float x_norm = static_cast<float>(coords[0]) / 255.0f - 0.5f;
        float y_norm = static_cast<float>(coords[1]) / 255.0f - 0.5f;

        float radius = std::sqrtf(x_norm * x_norm + y_norm * y_norm);
        float angle = std::atan2f(y_norm, x_norm);
        float x = 2.0f * octagon_radius * radius * std::cos(angle);
        float y = 2.0f * octagon_radius * radius * std::sin(angle);

        const auto rect_p1 = ImVec2(center_pos.x - half_size + x, center_pos.y - half_size - y);
        const auto rect_p2 = ImVec2(center_pos.x + half_size + x, center_pos.y + half_size - y);
        draw_list->AddRectFilled(rect_p1, rect_p2, color);
    };

    draw_list->AddRectFilled(cursor_pos, ImVec2(cursor_pos.x + size, cursor_pos.y + size),
                             colorWithAlpha(main_color, 0.1f));
    draw_list->AddRect(cursor_pos, ImVec2(cursor_pos.x + size, cursor_pos.y + size), background_color);
    draw_list->AddNgonFilled(center_pos, octagon_radius, background_color, 8);

    if (center.has_value()) {
        drawStickPoint(center.value(), calibration_color);
    }

    if (points.has_value()) {
        for (const auto& point : points.value()) {
            drawStickPoint(point, calibration_color);
        }
    }

    drawStickPoint(toPoint(stick_pos), main_color);

    ImGui::EndChild();
}
