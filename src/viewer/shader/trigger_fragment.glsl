#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform float fill;
uniform bool pressed;

const float inner_radius = 0.1;
float radius = inner_radius + border_width;

const float threshold = 0.75;
const float scale = 1.0 / threshold;

vec4 colorTrigger(vec2 pos, float fill, bool pressed);

void left() {
    vec2 pos = v_pos + vec2(0.5 - radius, 0.0);
    float dist = radius - sqrt((pos.x * pos.x) + (pos.y * pos.y));

    if (dist < 0.0 || ((v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale)
         && (dist >= border_width))) {
        discard;
    }
}

void middle() {
    if ((abs(v_pos.y) > radius) || ((abs(v_pos.y) <= radius - border_width)
         && (v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale))) {
        discard;
    }
}

void right() {
    vec2 pos = v_pos - vec2(0.5 - radius, 0.0);
    float dist = radius - sqrt((pos.x * pos.x) + (pos.y * pos.y));

    if (dist < 0.0 || ((v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale)
         && (dist >= border_width))) {
        discard;
    }
}

void main() {
    if (v_pos.x <= radius - 0.5) {
        left();
    } else if (v_pos.x >= 0.5 - radius) {
        right();
    } else {
        middle();
    }

    frag_color = colorTrigger(v_pos, fill, pressed);
}
