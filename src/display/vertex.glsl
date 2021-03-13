#version 330 core
layout (location = 0) in vec2 a_pos;

out vec2 v_pos;

void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
    v_pos = a_pos;
}
