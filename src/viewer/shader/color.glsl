#version 330 core
#define BUTTON_A 0
#define BUTTON_B 1
#define BUTTON_X 2
#define BUTTON_Y 3
#define BUTTON_START 4
#define BUTTON_Z 5
#define STICK_MAIN 6
#define STICK_C 7
#define TRIGGER_LEFT 8
#define TRIGGER_RIGHT 9
#define DPAD_UP 10
#define DPAD_LEFT 11
#define DPAD_RIGHT 12
#define DPAD_DOWN 13

#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

uniform vec2 resolution;
uniform float time;
uniform int which;

vec2 screen_pos = gl_FragCoord.xy / resolution;

const vec4 main_color = vec4(0.95, 0.95, 0.95, 1.0);

vec4 colorBackground() {
    return vec4(0.0);
}

vec4 colorButton(bool pressed) {
    switch (which) {
        case BUTTON_A:
            return vec4(0.0, 0.737, 0.556, 1.0);
        case BUTTON_B:
            return vec4(1.0, 0.0, 0.0, 1.0);
        case BUTTON_X:
        case BUTTON_Y:
        case BUTTON_START:
        case DPAD_UP:
        case DPAD_LEFT:
        case DPAD_RIGHT:
        case DPAD_DOWN:
            return main_color;
        case BUTTON_Z:
            return vec4(0.333, 0.0, 0.678, 1.0);
        default:
            return vec4(1.0);
    }
}

vec4 colorStick(vec2 stick_pos) {
    switch (which) {
        case STICK_MAIN:
            return main_color;
        case STICK_C:
            return vec4(1.0, 0.894, 0.0, 1.0);
        default:
            return vec4(1.0);
    }
}

vec4 colorTrigger(float fill, bool pressed) {
    return main_color;
}
