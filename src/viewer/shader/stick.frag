#version 330 core
in vec2 v_pos;
in vec2 v_tex_coord;
in float border_width;

layout (location = 0) out vec4 frag_color;

// octagon notch sdf
uniform sampler2D sdf_texture;
uniform vec2 pos;
uniform bool is_c_stick;

float radius = 0.225 * (is_c_stick ? 0.8 : 1.0);

vec4 colorStick(vec2 stick_pos);

void main() {
    vec2 center = v_pos + ((pos - 0.5) * 0.5);
    float dist = radius - sqrt((center.x * center.x) + (center.y * center.y));

    vec2 scaled_tex_coords = (v_tex_coord - 0.5) / 0.85 + 0.5;
    float sdf_dist = texture(sdf_texture, scaled_tex_coords).r;

    if ((dist < 0.0 && (sdf_dist < 0.5 - (4.0 * border_width) || sdf_dist >= 0.5))
         || (!is_c_stick && dist >= border_width)) {
        discard;
    }

    frag_color = colorStick(pos);
}
