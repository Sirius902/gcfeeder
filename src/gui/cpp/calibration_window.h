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

    template <std::convertible_to<ApplyCallback> StickApplyCallback,
              std::convertible_to<ApplyCallback> TriggerApplyCallback>
    CalibrationWindow(const GuiState& state, StickApplyCallback&& stick_apply_fn,
                      TriggerApplyCallback&& trigger_apply_fn)
        : state(state),
          stick_apply_fn(std::forward<StickApplyCallback>(stick_apply_fn)),
          trigger_apply_fn(std::forward<TriggerApplyCallback>(trigger_apply_fn)) {}

    bool isCalibrating() const { return is_calibrating.load(std::memory_order_acquire); }

    void updateInputs(Inputs inputs);
    void drawAndUpdate(const char* title, bool& open);

private:
    using clock = std::chrono::steady_clock;

    const GuiState& state;
    ApplyCallback stick_apply_fn;
    ApplyCallback trigger_apply_fn;

    std::mutex mutex;
    Inputs inputs{};
    std::atomic<bool> is_calibrating{false};

    std::optional<Point> main_stick_center;
    std::optional<Point> c_stick_center;
    std::vector<Point> main_stick_points;
    std::vector<Point> c_stick_points;

    std::array<std::optional<std::uint8_t>, 2> l_trigger_range;
    std::array<std::optional<std::uint8_t>, 2> r_trigger_range;

    bool view_calibration_data{false};
    bool a_was_pressed{false};

    void applyStickCalibration();
    void applyTriggerCalibration();

    bool shouldConfirm(const Inputs& inputs);
    void drawStickPopup(const Inputs& inputs);
    void drawTriggerPopup(const Inputs& inputs);

    static void drawStick(const char* str_id, Vec2 stick_pos, ImColor color,
                          std::optional<std::span<const Point>> points = std::nullopt,
                          std::optional<Point> center = std::nullopt);

    static void drawTrigger(const char* str_id, std::uint8_t value, char signifier, ImColor color,
                            std::optional<std::span<const std::uint8_t>> range = std::nullopt);
};
