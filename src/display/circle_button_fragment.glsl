#version 330 core
in vec2 v_pos;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform float radius;
uniform vec3 color;
uniform bool pressed;

void main() {
    float sq_dist = (radius * radius) - ((v_pos.x * v_pos.x) + (v_pos.y * v_pos.y));

    if (sq_dist < 0.0 || (!pressed && sq_dist >= border_width * border_width)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
