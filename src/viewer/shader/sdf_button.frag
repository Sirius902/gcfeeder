#version 330 core
#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

in vec2 v_Position;
in vec2 v_TexCoord;
in float v_BorderWidth;

layout (location = 0) out vec4 diffuseColor;

uniform sampler2D u_SdfTexture;
uniform bool u_Pressed;

vec4 buttonColor(bool pressed);

void main() {
    float dist = texture(u_SdfTexture, v_TexCoord).r;

    if (dist < 0.5 - (4.0 * v_BorderWidth) || (!u_Pressed && dist >= 0.5)) {
        discard;
    }

    diffuseColor = buttonColor(u_Pressed);
}
