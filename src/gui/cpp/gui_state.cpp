#include "gui_state.h"

GuiState::GuiState(UIContext& context, AppLog& log)
    : context(context), log(log), config(context.config_path, context.schema_str) {}
