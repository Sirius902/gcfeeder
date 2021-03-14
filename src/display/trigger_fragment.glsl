#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform vec3 color;
uniform float fill;

const float radius = 0.1;
const float hr = radius / 2.0;

const float threshold = 0.75;
const float scale = 1.0 / threshold;

void left() {
    vec2 pos = v_pos + vec2(0.5 - radius, 0.0);
    float sq_dist = (radius * radius) - ((pos.x * pos.x) + (pos.y * pos.y));

    if (sq_dist < 0.0 || ((v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale)
         && (sq_dist >= border_width * border_width))) {
        discard;
    }
}

void middle() {
    // That should definitely not be `border_width / 4.0` but since it works for this
    // radius we can just ignore that and scale it.
    if ((abs(v_pos.y) > radius) || ((abs(v_pos.y) <= radius - border_width / 4.0)
         && (v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale))) {
        discard;
    }
}

void right() {
    vec2 pos = v_pos - vec2(0.5 - radius, 0.0);
    float sq_dist = (radius * radius) - ((pos.x * pos.x) + (pos.y * pos.y));

    if (sq_dist < 0.0 || ((v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale)
         && (sq_dist >= border_width * border_width))) {
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

    frag_color = vec4(color, 1.0);
}
