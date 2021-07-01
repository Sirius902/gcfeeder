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
#define PAD_UP 10
#define PAD_LEFT 11
#define PAD_RIGHT 12
#define PAD_DOWN 13

#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

uniform vec2 resolution;
uniform float time;
uniform int which;

vec2 screen_pos = gl_FragCoord.xy / resolution;

vec4 wave_color = vec4((sin(time) + 1.0) / 2.0, screen_pos.y, 1.0, 1.0);

vec4 colorBackground() {
    return vec4(0.0);
}

vec4 colorButton(bool pressed) {
    return wave_color;
}

vec4 colorStick(vec2 stick_pos) {
    return wave_color;
}

vec4 colorTrigger(float fill, bool pressed) {
    return wave_color;
}
