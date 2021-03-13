#version 330 core
in vec2 v_pos;

layout (location = 0) out vec4 frag_color;

uniform vec2 center;
uniform float radius;
uniform vec3 color;
uniform bool pressed;

void main() {
    vec2 dist = v_pos - center;
    float sq_dist = (radius * radius) - ((dist.x * dist.x) + (dist.y * dist.y));
    float border_thickness = 0.25;

    if (sq_dist < 0.0 || (!pressed && sq_dist >= border_thickness * border_thickness)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
