#pragma once

#include <array>
#include <cstdint>
#include <filesystem>
#include <nlohmann/json.hpp>
#include <optional>
#include <string_view>
#include <type_traits>

#include "gui_main.h"

class Config {
public:
    using json = nlohmann::ordered_json;

    const std::filesystem::path json_path;
    const json schema;

    static constexpr std::string_view default_inversion_mapping = "oot-vc";
    static constexpr std::array<std::uint8_t, 2> default_stick_center{128, 128};
    static constexpr std::array<std::uint8_t, 2> default_trigger_range{0, 255};

    Config(const std::filesystem::path& json_path, std::string_view schema_str);

    bool isLoaded() const { return config.has_value(); }

    json& getJson() {
        static_assert(!std::is_const_v<decltype(config)> && !std::is_const_v<decltype(config)::value_type>);
        return const_cast<json&>(const_cast<const Config*>(this)->getJson());
    }
    const json& getJson() const { return config.value(); }

    json& getCurrentProfile() {
        static_assert(!std::is_const_v<decltype(config)> && !std::is_const_v<decltype(config)::value_type>);
        return const_cast<json&>(const_cast<const Config*>(this)->getCurrentProfile());
    }
    const json& getCurrentProfile() const;

    [[nodiscard]] static std::array<std::array<std::uint8_t, 2>, 8> defaultNotchPoints();
    [[nodiscard]] static json defaultStickCalibration();
    [[nodiscard]] static json defaultTriggerCalibration();

    void load();
    void save();

private:
    std::optional<json> config;
};
