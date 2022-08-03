#pragma once

#include <atomic>

#include "app_log.h"
#include "config.h"
#include "gui_main.h"

struct GuiState {
    UIContext& context;
    AppLog& log;
    Config config;
    std::atomic_bool feeder_needs_reload{true};

    GuiState(UIContext& context, AppLog& log);
};
