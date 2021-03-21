#version 330 core
in vec2 v_pos;
in vec2 v_tex_coord;
in float border_width;

layout (location = 0) out vec4 frag_color;

// octagon notch sdf
uniform sampler2D sdf_texture;
uniform vec3 color;
uniform vec2 pos;
uniform bool is_c_stick = false;

const float inner_radius = 0.5 * 0.25;
float radius = inner_radius * (is_c_stick ? 0.8 : 1.0) + border_width;

void main() {
    vec2 center = v_pos + ((pos - 0.5) * 0.35);
    float sq_dist = (radius * radius) - ((center.x * center.x) + (center.y * center.y));

    vec2 scaled_tex_coords = (v_tex_coord - 0.5) / 0.85 + 0.5;
    float sdf_dist = texture(sdf_texture, scaled_tex_coords).r;

    if ((sq_dist < 0.0 && (sdf_dist < 0.5 - border_width || sdf_dist >= 0.5))
         || (!is_c_stick && sq_dist >= border_width * border_width)) {
        discard;
    }

    frag_color = vec4(color, 1.0);
}
