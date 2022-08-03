#pragma once

#include <filesystem>
#include <nlohmann/json.hpp>
#include <optional>
#include <string_view>

#include "gui_main.h"

class Config {
public:
    using json = nlohmann::ordered_json;

    const std::filesystem::path json_path;
    const json schema;

    Config(const std::filesystem::path& json_path, std::string_view schema_str);

    bool isLoaded() { return config.has_value(); }
    json& getJson() { return config.value(); }

    void load();
    void save();

private:
    std::optional<json> config;
};
