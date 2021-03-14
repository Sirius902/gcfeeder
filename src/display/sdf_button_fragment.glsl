#version 330 core
in vec2 v_pos;
in vec2 v_tex_coord;

layout (location = 0) out vec4 frag_color;

uniform vec2 center;
uniform float size;
uniform vec3 color;
uniform bool pressed;
uniform sampler2D sdf_texture;

void main() {
    float border_thickness = 0.25;

    if (texture(sdf_texture, v_tex_coord).r < 0.5 || (!pressed && texture(sdf_texture, v_tex_coord).r > 0.5 + border_thickness)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
