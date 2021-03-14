#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform vec3 color;
uniform float fill;

const float radius = 0.5;

const float threshold = 0.75;
const float scale = 1.0 / threshold;

void main() {
    float sq_dist = (radius * radius) - ((v_pos.x * v_pos.x) + (v_pos.y * v_pos.y));

    if (sq_dist < 0.0 || ((sq_dist >= border_width * border_width)
         && (v_pos.x + 0.5 > clamp(fill, 0.0, threshold) * scale))) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
