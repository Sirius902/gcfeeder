#pragma once

#define GLFW_INCLUDE_NONE
#include <GLFW/glfw3.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct UIContext {
    char* ttf_ptr;
    size_t ttf_len;
    const char* ini_path;
    GLFWwindow* window;
    const char* glsl_version;
} UIContext;

int runImGui(UIContext* context);
void addLogMessage(const char* message);

#ifdef __cplusplus
}
#endif
