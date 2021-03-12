#version 330 core
layout (location = 0) in vec2 v_pos;
layout (location = 1) in vec3 v_color;

out vec3 vert_color;

void main()
{
    gl_Position = vec4(v_pos, 0.0, 1.0);
    vert_color = v_color;
}
