#version 330 core
in vec2 v_tex_coord;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform sampler2D sdf_texture;
uniform vec3 color;
uniform bool pressed;

void main() {
    float dist = texture(sdf_texture, v_tex_coord).r;

    if (dist < 0.5 - border_width || (!pressed && dist >= 0.5)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
