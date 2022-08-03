#include "config.h"

#include <fmt/core.h>

#include <cstddef>
#include <cstring>
#include <exception>
#include <fstream>

namespace fs = std::filesystem;

Config::Config(const fs::path& json_path, std::string_view schema_str)
    : json_path(json_path), schema(json::parse(schema_str)) {}

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
