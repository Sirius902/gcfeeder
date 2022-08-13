#include "config.h"

#include <fmt/core.h>

#include <cmath>
#include <cstddef>
#include <cstring>
#include <exception>
#include <fstream>
#include <numbers>
#include <string>

#include "util.h"

namespace fs = std::filesystem;

Config::Config(const fs::path& json_path, std::string_view schema_str)
    : json_path(json_path), schema(json::parse(schema_str)) {}

const Config::json& Config::getCurrentProfile() const {
    const auto& config = getJson();
    const auto& current_profile_str = config.at("current_profile").get_ref<const std::string&>();

    for (const auto& [_, value] : config.at("profiles").items()) {
        if (value.at("name").get_ref<const std::string&>() == current_profile_str) {
            return value.at("config");
        }
    }

    throw std::runtime_error(fmt::format("Profile not found: \"{}\"", current_profile_str));
}

std::array<std::array<std::uint8_t, 2>, 8> Config::defaultNotchPoints() {
    constexpr auto pi = std::numbers::pi;

    std::array<std::array<std::uint8_t, 2>, 8> points;
    for (std::size_t i = 0; i < points.size(); i++) {
        double angle = pi / 2 - (i * pi / 4);
        std::uint8_t x = util::lossy_cast<std::uint8_t>(127.0f * std::cos(angle) + 128.0f);
        std::uint8_t y = util::lossy_cast<std::uint8_t>(127.0f * std::sin(angle) + 128.0f);
        points[i] = {x, y};
    }
    return points;
}

Config::json Config::defaultStickCalibration() {
    json obj;
    auto& main_stick = obj["main_stick"];
    main_stick["notch_points"] = defaultNotchPoints();
    main_stick["stick_center"] = default_stick_center;
    obj["c_stick"] = main_stick;
    return obj;
}

Config::json Config::defaultTriggerCalibration() {
    json obj;
    auto& l_trigger = obj["l_trigger"];
    l_trigger["min"] = 0;
    l_trigger["max"] = 255;
    obj["r_trigger"] = l_trigger;
    return obj;
}

void Config::load() {
    std::ifstream config_file(json_path.c_str());
    if (!config_file.is_open()) {
        throw std::runtime_error(
            fmt::format("Failed to open for reading \"{}\": {}", json_path.string().c_str(), std::strerror(errno)));
    }

    if (config.has_value()) {
        config->clear();
    } else {
        config.emplace();
    }

    config_file >> config.value();
}

using FormatCallback = std::size_t (*)(const char* bytes_ptr, std::size_t bytes_len, void* userdata);
extern "C" void formatConfigJson(const char* json_ptr, std::size_t json_len, FormatCallback callback, void* userdata);

void Config::save() {
    if (config.has_value()) {
        std::ofstream config_file(json_path.c_str());
        if (!config_file.is_open()) {
            throw std::runtime_error(
                fmt::format("Failed to open for writing \"{}\": {}", json_path.string().c_str(), std::strerror(errno)));
        }

        const auto dump = config->dump();
        formatConfigJson(
            dump.c_str(), dump.size(),
            [](const char* bytes_ptr, std::size_t bytes_len, void* userdata) {
                std::ofstream& config_file = *static_cast<std::ofstream*>(userdata);
                config_file << std::string_view{bytes_ptr, bytes_len};
                return bytes_len;
            },
            static_cast<void*>(&config_file));
    } else {
        throw std::runtime_error("Config::save(): config was null");
    }
}
