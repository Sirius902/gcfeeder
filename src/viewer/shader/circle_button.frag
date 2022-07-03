#version 330 core
#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

in vec2 v_Position;
in float v_BorderWidth;

layout (location = 0) out vec4 diffuseColor;

uniform bool u_Pressed;

float radius = 0.1;

vec4 buttonColor(bool pressed);

void main() {
    float dist = radius - sqrt((v_Position.x * v_Position.x) + (v_Position.y * v_Position.y));

    if (dist < 0.0 || (!u_Pressed && dist >= v_BorderWidth)) {
        discard;
    }

    diffuseColor = buttonColor(u_Pressed);
}
