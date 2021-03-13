#version 330 core
in vec2 v_pos;

layout (location = 0) out vec4 frag_color;

uniform vec2 center;
uniform float size;
uniform vec3 color;
uniform bool pressed;

void main() {
    vec2 dist = (v_pos + size * (3.0 / 10.0)) - center;
    // (x^2 + y^2)^2
    float left = pow((dist.x * dist.x) + (dist.y * dist.y), 2.0);
    // a(x^3 + y^3)
    float right = size * (pow(dist.x, 3.0) + pow(dist.y, 3.0));

    float s = 0.945;
    vec2 smaller_dist = (v_pos + size * s * (3.0 / 10.0)) - center;
    float smaller_left = pow((smaller_dist.x * smaller_dist.x) + (smaller_dist.y * smaller_dist.y), 2.0);
    float smaller_right = size * s * (pow(smaller_dist.x, 3.0) + pow(smaller_dist.y, 3.0));

    if (left > right || (!pressed && smaller_left <= smaller_right)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
