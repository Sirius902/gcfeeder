#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_tex_coord;

out vec2 v_pos;
out vec2 v_tex_coord;
out float border_width;

uniform vec2 window_size;
uniform mat4 model;
uniform float scale = 1.0;

void main() {
    float sx;
    float sy;

    if (window_size.x > window_size.y * 2.0) {
        sx = (window_size.y * 2.0) / window_size.x;
        sy = 2.0;
    } else {
        sx = 1.0;
        sy = window_size.x / window_size.y;
    }

    mat4 aspect_matrix = mat4(
        sx, 0.0, 0.0, 0.0,
        0.0, sy, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );

    gl_Position = aspect_matrix * model * vec4(a_pos, 0.0, 1.0);
    v_pos = a_pos;
    v_tex_coord = a_tex_coord;
    border_width = 0.025 / scale;
}
