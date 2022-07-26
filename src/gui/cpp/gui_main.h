#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CUIContext {
    char* ttf_ptr;
    size_t ttf_len;
    const char* exe_dir_path;
    GLFWwindow* window;
    const char* glsl_version;
    const char* program_version;
    const char* usercontent_url;
    const char* config_path;
    const char* schema_rel_path;
    const char* schema_file_ptr;
    size_t schema_file_len;
} CUIContext;

int runImGui(CUIContext* c_context);
void addLogMessage(const char* message);
int isFeederReloadNeeded(void);
void notifyFeederReload(void);

#ifdef __cplusplus
}
#endif

#ifdef __cplusplus
#include <filesystem>
#include <span>
#include <string_view>

struct UIContext {
    std::span<char> ttf;
    std::filesystem::path exe_dir;
    GLFWwindow* window;
    std::string_view glsl_version;
    std::string_view program_version;
    std::string_view usercontent_url;
    std::filesystem::path config_path;
    std::string_view schema_rel_path_str;
    std::string_view schema_str;

    UIContext(const CUIContext& ctx)
        : ttf(ctx.ttf_ptr, ctx.ttf_len),
          exe_dir(ctx.exe_dir_path),
          window(ctx.window),
          glsl_version(ctx.glsl_version),
          program_version(ctx.program_version),
          usercontent_url(ctx.usercontent_url),
          config_path(exe_dir / ctx.config_path),
          schema_rel_path_str(ctx.schema_rel_path),
          schema_str(ctx.schema_file_ptr, ctx.schema_file_len) {}
};
#endif
