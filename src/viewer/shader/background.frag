#version 330 core
layout (location = 0) out vec4 frag_color;

vec4 colorBackground();

void main() {
    frag_color = colorBackground();
}
