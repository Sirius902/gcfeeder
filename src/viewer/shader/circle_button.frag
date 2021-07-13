#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform bool pressed;

float radius = 0.1;

vec4 colorButton(bool pressed);

void main() {
    float dist = radius - sqrt((v_pos.x * v_pos.x) + (v_pos.y * v_pos.y));

    if (dist < 0.0 || (!pressed && dist >= border_width)) {
        discard;
    }

    frag_color = colorButton(pressed);
}
