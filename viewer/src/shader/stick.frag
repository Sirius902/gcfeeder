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

// octagon notch sdf
uniform sampler2D u_SdfTexture;
uniform vec2 u_Pos;
uniform bool u_IsCStick;

float radius = 0.225 * (u_IsCStick ? 0.8 : 1.0);

vec4 stickColor(vec2 stickPos);

void main() {
    vec2 center = v_Position + ((u_Pos - 0.5) * 0.5);
    float dist = radius - sqrt((center.x * center.x) + (center.y * center.y));

    vec2 scaledTexCoords = (v_TexCoord - 0.5) / 0.85 + 0.5;
    float sdfDist = texture(u_SdfTexture, scaledTexCoords).r;

    if ((dist < 0.0 && (sdfDist < 0.5 - (4.0 * v_BorderWidth) || sdfDist >= 0.5))
         || (!u_IsCStick && dist >= v_BorderWidth)) {
        discard;
    }

    diffuseColor = stickColor(u_Pos);
}
