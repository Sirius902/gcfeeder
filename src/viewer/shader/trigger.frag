#version 330 core
#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

in vec2 v_Position;
in float v_BorderWidth;

layout (location = 0) out vec4 diffuseColor;

uniform float u_Fill;
uniform bool u_Pressed;

const float innerRadius = 0.1;
float radius = innerRadius + v_BorderWidth;

const float threshold = 0.75;
const float scale = 1.0 / threshold;

vec4 colorTrigger(float fill, bool pressed);

void left() {
    vec2 pos = v_Position + vec2(0.5 - radius, 0.0);
    float dist = radius - sqrt((pos.x * pos.x) + (pos.y * pos.y));

    if (dist < 0.0 || ((v_Position.x + 0.5 > clamp(u_Fill, 0.0, threshold) * scale)
         && (dist >= v_BorderWidth))) {
        discard;
    }
}

void middle() {
    if ((abs(v_Position.y) > radius) || ((abs(v_Position.y) <= radius - v_BorderWidth)
         && (v_Position.x + 0.5 > clamp(u_Fill, 0.0, threshold) * scale))) {
        discard;
    }
}

void right() {
    vec2 pos = v_Position - vec2(0.5 - radius, 0.0);
    float dist = radius - sqrt((pos.x * pos.x) + (pos.y * pos.y));

    if (dist < 0.0 || ((v_Position.x + 0.5 > clamp(u_Fill, 0.0, threshold) * scale)
         && (dist >= v_BorderWidth))) {
        discard;
    }
}

void main() {
    if (v_Position.x <= radius - 0.5) {
        left();
    } else if (v_Position.x >= 0.5 - radius) {
        right();
    } else {
        middle();
    }

    diffuseColor = colorTrigger(u_Fill, u_Pressed);
}
