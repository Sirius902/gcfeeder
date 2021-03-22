#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_tex_coord;

out vec2 v_pos;
out vec2 v_tex_coord;
out float border_width;

uniform mat4 model;
uniform float scale = 1.0;

void main() {
    gl_Position = model * vec4(a_pos, 0.0, 1.0);
    v_pos = a_pos;
    v_tex_coord = a_tex_coord;
    border_width = 0.025 / scale;
}
