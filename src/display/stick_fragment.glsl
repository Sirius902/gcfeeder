#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform vec3 color;
uniform vec2 pos;
uniform bool is_c_stick = false;

const float inner_radius = 0.5 * 0.25;
float radius = inner_radius + border_width;

void main() {
    vec2 center = v_pos + ((pos - 0.5) / 2.0);
    float sq_dist = (radius * radius) - ((center.x * center.x) + (center.y * center.y));

    if (sq_dist < 0.0 || (!is_c_stick && sq_dist >= border_width * border_width)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
