#version 330 core
#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

layout (location = 0) out vec4 diffuseColor;

vec4 backgroundColor();

void main() {
    diffuseColor = backgroundColor();
}
