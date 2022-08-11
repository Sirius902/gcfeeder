#pragma once

#include <array>
#include <atomic>
#include <concepts>
#include <cstdint>
#include <functional>
#include <mutex>
#include <nlohmann/json.hpp>
#include <optional>
#include <utility>
#include <vector>

#define IMGUI_IMPL_OPENGL_LOADER_CUSTOM
#include <imgui.h>

#include "config.h"
#include "gui_main.h"

class CalibrationWindow {
public:
    using ApplyFunc = std::function<void(Config::json&&)>;

    template <std::convertible_to<ApplyFunc> ApplyFuncT>
    CalibrationWindow(ApplyFuncT&& apply_fn) : apply_fn(std::forward<ApplyFuncT>(apply_fn)) {}

    bool isCalibrating() const { return is_calibrating.load(std::memory_order_acquire); }

    void updateInputs(Inputs inputs);
    void drawAndUpdate(const char* title, bool& open);

private:
    using clock = std::chrono::steady_clock;
    using Point = std::array<std::uint8_t, 2>;

    ApplyFunc apply_fn;

    std::mutex mutex;
    Inputs inputs{};
    std::atomic<bool> is_calibrating{false};

    std::optional<Point> main_stick_center;
    std::optional<Point> c_stick_center;
    std::vector<Point> main_stick_points;
    std::vector<Point> c_stick_points;

    bool was_pressed{false};

    void applyCalibration();
    void drawPopup(const Inputs& inputs);
};
