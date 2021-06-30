#version 330 core
in vec2 v_pos;
in vec2 v_tex_coord;
in float border_width;

layout (location = 0) out vec4 frag_color;

uniform sampler2D sdf_texture;
uniform bool pressed;

vec4 colorButton(vec2 pos, bool pressed);

void main() {
    float dist = texture(sdf_texture, v_tex_coord).r;

    if (dist < 0.5 - (4.0 * border_width) || (!pressed && dist >= 0.5)) {
        discard;
    }

    frag_color = colorButton(v_pos, pressed);
}
