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

struct TriggerCalibration {
    std::array<std::uint8_t, 2> range;

    TriggerCalibration() = default;

    TriggerCalibration(const json& data) {
        const auto& min = data.at("min").get<json::number_unsigned_t>();
        const auto& max = data.at("max").get<json::number_unsigned_t>();

        if (min >= max) throw std::runtime_error{"TriggerCalibration min >= max"};

        range[0] = util::lossy_cast<std::uint8_t>(min);
        range[1] = util::lossy_cast<std::uint8_t>(max);
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

    if (ImGui::Button("Calibrate Sticks")) {
        if (!is_calibrating.load(std::memory_order_relaxed)) {
            ImGui::OpenPopup("##stick_calibration_popup");
            is_calibrating.store(true, std::memory_order_release);
        }
    }

    ImGui::SameLine();

    if (ImGui::Button("Calibrate Triggers")) {
        if (!is_calibrating.load(std::memory_order_relaxed)) {
            ImGui::OpenPopup("##trigger_calibration_popup");
            is_calibrating.store(true, std::memory_order_release);
        }
    }

    ImGui::Checkbox("View Calibration Data", &view_calibration_data);

    if (!(inputs.active_stages & STAGE_RAW)) ImGui::EndDisabled();

    drawStickPopup(inputs);
    drawTriggerPopup(inputs);

    StickCalibration main_stick_calibration;
    std::optional<std::span<Point>> main_stick_points;
    std::optional<Point> main_stick_center;

    StickCalibration c_stick_calibration;
    std::optional<std::span<Point>> c_stick_points;
    std::optional<Point> c_stick_center;

    TriggerCalibration l_trigger_calibration;
    std::optional<std::span<std::uint8_t>> l_trigger_range;

    TriggerCalibration r_trigger_calibration;
    std::optional<std::span<std::uint8_t>> r_trigger_range;

    if (state.config.isLoaded() && view_calibration_data) {
        const auto& profile = state.config.getCurrentProfile();
        const auto& calibration = profile.at("calibration");
        const auto& stick_data = calibration.at("stick_data");
        const auto& trigger_data = calibration.at("trigger_data");

        if (!stick_data.is_null()) {
            main_stick_calibration = StickCalibration(stick_data.at("main_stick"));
            main_stick_points = std::span(main_stick_calibration.notch_points);
            main_stick_center = main_stick_calibration.stick_center;

            c_stick_calibration = StickCalibration(stick_data.at("c_stick"));
            c_stick_points = std::span(c_stick_calibration.notch_points);
            c_stick_center = c_stick_calibration.stick_center;
        }

        if (!trigger_data.is_null()) {
            l_trigger_calibration = TriggerCalibration(trigger_data.at("l_trigger"));
            l_trigger_range = l_trigger_calibration.range;

            r_trigger_calibration = TriggerCalibration(trigger_data.at("r_trigger"));
            r_trigger_range = r_trigger_calibration.range;
        }
    }

    if (inputs.active_stages & STAGE_RAW) {
        const auto& stage = inputs.stages.raw;
        ImGui::TextUnformatted("Raw");
        drawStick("main_raw", stage.main_stick, main_stick_color, main_stick_points, main_stick_center);
        ImGui::SameLine();
        drawStick("c_raw", stage.c_stick, c_stick_color, c_stick_points, c_stick_center);

        ImGui::SameLine();

        drawTrigger("l_trigger_raw", stage.l_trigger, 'L', main_stick_color, l_trigger_range);
        ImGui::SameLine();
        drawTrigger("r_trigger_raw", stage.r_trigger, 'R', main_stick_color, r_trigger_range);
    }

    if (inputs.active_stages & STAGE_MAPPED) {
        const auto& stage = inputs.stages.mapped;
        ImGui::TextUnformatted("Mapped");
        drawStick("main_mapped", stage.main_stick, main_stick_color, main_stick_points, main_stick_center);
        ImGui::SameLine();
        drawStick("c_mapped", stage.c_stick, c_stick_color, c_stick_points, c_stick_center);

        ImGui::SameLine();

        drawTrigger("l_trigger_mapped", stage.l_trigger, 'L', main_stick_color, l_trigger_range);
        ImGui::SameLine();
        drawTrigger("r_trigger_mapped", stage.r_trigger, 'R', main_stick_color, r_trigger_range);
    }

    if (inputs.active_stages & STAGE_CALIBRATED) {
        static const auto points = Config::defaultNotchPoints();
        const auto points_span = view_calibration_data
                                     ? std::optional{std::span(points.data(), points.data() + points.size())}
                                     : std::nullopt;
        const auto center = view_calibration_data ? std::optional{Config::default_stick_center} : std::nullopt;

        const auto& stage = inputs.stages.calibrated;
        ImGui::TextUnformatted("Calibrated");
        drawStick("main_calibrated", stage.main_stick, main_stick_color, points_span, center);
        ImGui::SameLine();
        drawStick("c_calibrated", stage.c_stick, c_stick_color, points_span, center);

        ImGui::SameLine();

        const auto& range = Config::default_trigger_range;
        const auto range_span =
            view_calibration_data ? std::optional{std::span(range.data(), range.data() + range.size())} : std::nullopt;

        drawTrigger("l_trigger_calibrated", stage.l_trigger, 'L', main_stick_color, range_span);
        ImGui::SameLine();
        drawTrigger("r_trigger_calibrated", stage.r_trigger, 'R', main_stick_color, range_span);
    }

    if (inputs.active_stages & STAGE_SCALED) {
        const auto& stage = inputs.stages.scaled;
        ImGui::TextUnformatted("Scaled");
        drawStick("main_scaled", stage.main_stick, main_stick_color);
        ImGui::SameLine();
        drawStick("c_scaled", stage.c_stick, c_stick_color);

        ImGui::SameLine();

        drawTrigger("l_trigger_scaled", stage.l_trigger, 'L', main_stick_color);
        ImGui::SameLine();
        drawTrigger("r_trigger_scaled", stage.r_trigger, 'R', main_stick_color);
    }

    ImGui::End();
}

void CalibrationWindow::applyStickCalibration() {
    json calibration;

    auto& main_stick = calibration["main_stick"];
    main_stick["notch_points"] = main_stick_points;
    main_stick["stick_center"] = main_stick_center.value();

    auto& c_stick = calibration["c_stick"];
    c_stick["notch_points"] = c_stick_points;
    c_stick["stick_center"] = c_stick_center.value();

    stick_apply_fn(std::move(calibration));
}

void CalibrationWindow::applyTriggerCalibration() {
    json calibration;

    auto& l_trigger = calibration["l_trigger"];
    l_trigger["min"] = l_trigger_range[0].value();
    l_trigger["max"] = l_trigger_range[1].value();

    auto& r_trigger = calibration["r_trigger"];
    r_trigger["min"] = r_trigger_range[0].value();
    r_trigger["max"] = r_trigger_range[1].value();

    trigger_apply_fn(std::move(calibration));
}

bool CalibrationWindow::shouldConfirm(const Inputs& inputs) {
    if (inputs.a_pressed && !a_was_pressed) {
        a_was_pressed = true;
        return true;
    } else if (!inputs.a_pressed) {
        a_was_pressed = false;
    }

    return false;
}

void CalibrationWindow::drawStickPopup(const Inputs& inputs) {
    static auto endCalibration = [this]() {
        is_calibrating.store(false, std::memory_order_release);
        main_stick_center.reset();
        c_stick_center.reset();
        main_stick_points.clear();
        c_stick_points.clear();
    };

    static auto drawCalibrationProgress = [&]() {
        bool is_main_stick = true;
        while (true) {
            std::string_view stick_name = is_main_stick ? "main " : "C-";
            auto& center = is_main_stick ? main_stick_center : c_stick_center;
            auto& points = is_main_stick ? main_stick_points : c_stick_points;
            const auto& stick = is_main_stick ? inputs.stages.raw.main_stick : inputs.stages.raw.c_stick;
            const auto& color = is_main_stick ? main_stick_color : c_stick_color;

            drawStick(fmt::format("{} stick", stick_name).c_str(), stick, color, points, center);

            if (!center.has_value()) {
                ImGui::TextUnformatted(fmt::format("Center {}stick and press A", stick_name).c_str());
                if (shouldConfirm(inputs)) {
                    center = {stick.x, stick.y};
                }
                return;
            }

            for (std::size_t notch = 0; notch < notch_names.size(); notch++) {
                const auto& notch_name = notch_names.at(notch);

                if (notch >= points.size()) {
                    ImGui::TextUnformatted(
                        fmt::format("Move {}stick to center then to {} then press A", stick_name, notch_name).c_str());
                    if (shouldConfirm(inputs)) {
                        points.push_back({stick.x, stick.y});
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

    if (ImGui::BeginPopup("##stick_calibration_popup")) {
        if (ImGui::Button("Cancel")) {
            ImGui::CloseCurrentPopup();
            endCalibration();
        }

        ImGui::Separator();

        ImGui::BeginChild("scrolling", ImVec2(500.0f, 300.0f), false, ImGuiWindowFlags_HorizontalScrollbar);

        ImGui::TextUnformatted("Calibrating sticks...");
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

void CalibrationWindow::drawTriggerPopup(const Inputs& inputs) {
    static auto endCalibration = [this]() {
        is_calibrating.store(false, std::memory_order_release);
        l_trigger_range = {};
        r_trigger_range = {};
    };

    static auto drawCalibrationProgress = [&]() {
        bool is_left_stick = true;
        while (true) {
            std::string_view trigger_name = is_left_stick ? "left" : "right";
            auto& range = is_left_stick ? l_trigger_range : r_trigger_range;
            const auto& value = is_left_stick ? inputs.stages.raw.l_trigger : inputs.stages.raw.r_trigger;
            const auto& color = main_stick_color;

            std::size_t nullopt_count = 0;
            for (std::size_t i = 0; i < range.size(); i++) {
                if (!range[i].has_value()) {
                    nullopt_count++;
                }
            }

            const auto bounds_to_draw = std::to_array({
                range[0].value_or(0),
                range[1].value_or(0),
            });
            const auto bounds_to_draw_span =
                std::optional{std::span(bounds_to_draw.data(), bounds_to_draw.data() + (range.size() - nullopt_count))};

            drawTrigger(fmt::format("{} trigger", trigger_name).c_str(), value, is_left_stick ? 'L' : 'R', color,
                        bounds_to_draw_span);

            for (std::size_t i = 0; i < range.size(); i++) {
                if (!range[i].has_value()) {
                    if (i == 0) {
                        ImGui::TextUnformatted(
                            fmt::format("Completely release {} trigger then press A", trigger_name).c_str());
                    } else {
                        ImGui::TextUnformatted(
                            fmt::format("Press {} trigger all the way in then press A", trigger_name).c_str());
                    }

                    if (shouldConfirm(inputs)) {
                        range[i] = value;
                    }
                    return;
                }
            }

            if (is_left_stick) {
                is_left_stick = false;
            } else {
                break;
            }
        }
    };

    static auto isCalibrationFinished = [this]() { return r_trigger_range[1].has_value(); };

    if (ImGui::BeginPopup("##trigger_calibration_popup")) {
        if (ImGui::Button("Cancel")) {
            ImGui::CloseCurrentPopup();
            endCalibration();
        }

        ImGui::Separator();

        ImGui::BeginChild("scrolling", ImVec2(500.0f, 300.0f), false, ImGuiWindowFlags_HorizontalScrollbar);

        ImGui::TextUnformatted("Calibrating triggers...");
        drawCalibrationProgress();

        if (isCalibrationFinished()) {
            ImGui::Separator();

            ImGui::TextUnformatted("Calibration finished. Apply to config editor profile?");
            if (ImGui::Button("Apply")) {
                applyTriggerCalibration();
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

void CalibrationWindow::drawStick(const char* str_id, Vec2 stick_pos, ImColor color,
                                  std::optional<std::span<const Point>> points, std::optional<Point> center) {
    static constexpr auto size = 60.0f;
    static constexpr auto octagon_radius = size * 0.4f;
    const auto main_color = colorWithAlpha(color, 1.0f);
    const auto background_color = colorWithAlpha(main_color, 0.6f);
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

    const auto outer_rect_p2 = ImVec2(cursor_pos.x + size, cursor_pos.y + size);
    draw_list->AddRectFilled(cursor_pos, outer_rect_p2, colorWithAlpha(background_color, 0.1f));
    draw_list->AddRect(cursor_pos, outer_rect_p2, background_color);
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

void CalibrationWindow::drawTrigger(const char* str_id, std::uint8_t value, char signifier, ImColor color,
                                    std::optional<std::span<const std::uint8_t>> range) {
    constexpr auto size = ImVec2(15.0f, 60.0f);
    const auto main_color = colorWithAlpha(color, 1.0f);
    const auto background_color = colorWithAlpha(color, 0.6f);
    const auto calibration_color =
        ImColor(1.0f - main_color.Value.x, 1.0f - main_color.Value.y, 1.0f - main_color.Value.z);
    const auto signifier_color = ImColor(std::lerp(background_color.Value.x, calibration_color.Value.x, 0.5f),
                                         std::lerp(background_color.Value.y, calibration_color.Value.y, 0.5f),
                                         std::lerp(background_color.Value.z, calibration_color.Value.z, 0.5f));

    ImGui::BeginChild(str_id, size);

    const auto cursor_pos = ImGui::GetCursorScreenPos();

    ImDrawList* draw_list = ImGui::GetWindowDrawList();

    const auto outer_rect_p2 = ImVec2(cursor_pos.x + size.x, cursor_pos.y + size.y);
    draw_list->AddRectFilled(cursor_pos, outer_rect_p2, colorWithAlpha(background_color, 0.1f));
    draw_list->AddRect(cursor_pos, outer_rect_p2, background_color);

    const auto value_scaled = static_cast<float>(value) / 255.0f * size.y;
    const auto inner_rect_p1 = ImVec2(cursor_pos.x, cursor_pos.y + size.y);
    const auto inner_rect_p2 = ImVec2(cursor_pos.x + size.x, cursor_pos.y + size.y - value_scaled);
    draw_list->AddRectFilled(inner_rect_p1, inner_rect_p2, main_color);

    if (range.has_value()) {
        for (const auto& bound : range.value()) {
            constexpr auto bound_height = size.y / 20.0f;
            const auto y = std::clamp(1.0f - (static_cast<float>(bound) / 255.0f), 0.01f, 0.99f) * size.y;

            const auto p1 = ImVec2(cursor_pos.x, cursor_pos.y + 0.5f * bound_height + y);
            const auto p2 = ImVec2(cursor_pos.x + size.x, cursor_pos.y - 0.5f * bound_height + y);
            draw_list->AddRectFilled(p1, p2, calibration_color);
        }
    }

    const auto signifer_size = ImGui::CalcTextSize(&signifier, &signifier + 1);
    const auto signifier_pos =
        ImVec2(cursor_pos.x + 0.5f * (size.x - signifer_size.x), cursor_pos.y + 0.5f * (size.y - signifer_size.y));
    draw_list->AddText(signifier_pos, signifier_color, &signifier, &signifier + 1);

    ImGui::EndChild();
}
