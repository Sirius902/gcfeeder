#version 330 core
layout (location = 0) in vec2 a_Position;
layout (location = 1) in vec2 a_TexCoord;

out vec2 v_Position;
out vec2 v_TexCoord;
out float v_BorderWidth;

uniform mat4 u_Projection;
uniform mat4 u_Model;
uniform float u_Scale = 1.0;

void main() {
    gl_Position = u_Projection * u_Model * vec4(a_Position, 0.0, 1.0);
    v_Position = a_Position;
    v_TexCoord = a_TexCoord;
    v_BorderWidth = 0.025 / u_Scale;
}
