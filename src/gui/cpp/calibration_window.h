#pragma once

#include <array>
#include <atomic>
#include <concepts>
#include <cstdint>
#include <functional>
#include <mutex>
#include <nlohmann/json.hpp>
#include <optional>
#include <span>
#include <utility>
#include <vector>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

#include "config.h"
#include "gui_main.h"
#include "gui_state.h"

class CalibrationWindow {
public:
    using Point = std::array<std::uint8_t, 2>;
    using ApplyCallback = std::function<void(Config::json&&)>;

    template <std::convertible_to<ApplyCallback> ApplyCallbackT>
    CalibrationWindow(const GuiState& state, ApplyCallbackT&& apply_fn)
        : state(state), apply_fn(std::forward<ApplyCallbackT>(apply_fn)) {}

    bool isCalibrating() const { return is_calibrating.load(std::memory_order_acquire); }

    void updateInputs(Inputs inputs);
    void drawAndUpdate(const char* title, bool& open);

private:
    using clock = std::chrono::steady_clock;

    const GuiState& state;
    ApplyCallback apply_fn;

    std::mutex mutex;
    Inputs inputs{};
    std::atomic<bool> is_calibrating{false};

    std::optional<Point> main_stick_center;
    std::optional<Point> c_stick_center;
    std::vector<Point> main_stick_points;
    std::vector<Point> c_stick_points;

    bool view_calibration_points{false};
    bool was_pressed{false};

    void applyCalibration();
    void drawPopup(const Inputs& inputs);

    static void drawStick(const char* str_id, Vec2 stick_pos, ImColor main_color,
                          std::optional<std::span<Point>> points = std::nullopt,
                          std::optional<Point> center = std::nullopt);
};
