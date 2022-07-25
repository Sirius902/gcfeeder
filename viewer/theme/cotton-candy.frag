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

uniform vec2 u_Resolution;
uniform float u_Time;
uniform int u_Which;

vec2 screenPos = gl_FragCoord.xy / u_Resolution;

vec4 waveColor = vec4((sin(u_Time) + 1.0) / 2.0, screenPos.y, 1.0, 1.0);

vec4 backgroundColor() {
    return vec4(0.0);
}

vec4 buttonColor(bool pressed) {
    return waveColor;
}

vec4 stickColor(vec2 stickPos) {
    return waveColor;
}

vec4 triggerColor(float fill, bool pressed) {
    return waveColor;
}
