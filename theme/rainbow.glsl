#version 330 core
#define BUTTON_A 0
#define BUTTON_B 1
#define BUTTON_X 2
#define BUTTON_Y 3
#define BUTTON_START 4
#define BUTTON_Z 5
#define STICK_MAIN 6
#define STICK_C 7
#define TRIGGER_LEFT 8
#define TRIGGER_RIGHT 9
#define DPAD_UP 10
#define DPAD_LEFT 11
#define DPAD_RIGHT 12
#define DPAD_DOWN 13

#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
#else
    precision mediump float;
#endif

uniform vec2 resolution;
uniform float time;
uniform int which;

vec2 screen_pos = gl_FragCoord.xy / resolution;

const vec2 d65_cie = vec2(0.31271, 0.32902);
const vec3 d65 = vec3((1.0 / d65_cie.y) * d65_cie.x, 1.0, (1.0 / d65_cie.y) * (1.0 - d65_cie.x - d65_cie.y));

const vec2 sr = vec2(0.640, 0.330);
const vec2 sg = vec2(0.300, 0.600);
const vec2 sb = vec2(0.150, 0.060);
const vec3 rv = vec3(sr.x / sr.y, 1.0, (1.0 - sr.x - sr.y) / sr.y);
const vec3 gv = vec3(sg.x / sg.y, 1.0, (1.0 - sg.x - sg.y) / sg.y);
const vec3 bv = vec3(sb.x / sb.y, 1.0, (1.0 - sb.x - sb.y) / sb.y);
const mat3 standard = inverse(mat3(rv, gv, bv));

const vec3 st = standard * d65;

const vec3 str = st.x * rv;
const vec3 stg = st.y * gv;
const vec3 stb = st.z * bv;

const mat3 m = mat3(str, stg, stb);
const mat3 minv = inverse(m);

float gamma(float u) {
    if (u <= 0.0031308) {
	    return 12.92 * u;
	} else {
	    return (1.055 * pow(u, 1.0 / 2.4)) - 0.055;
	}
}

const float delta = 6.0 / 29.0;

float inversef(float t) {
    if (t > delta) {
	    return pow(t, 3.0);
	} else {
	    return 3 * pow(delta, 2.0) * (t - (4.0 / 29.0));
	}
}

vec3 rgbToSrgb(vec3 c) {
    return vec3(gamma(c.x), gamma(c.y), gamma(c.z));
}

vec3 xyzToRgb(vec3 c) {
    return minv * c;
}

vec3 labToXyz(vec3 c) {
    float x = d65.x * inversef(((c.x + 16.0) / 116.0) + (c.y / 500.0));
	float y = d65.y * inversef((c.x + 16.0) / 116.0);
	float z = d65.z * inversef(((c.x + 16.0) / 116.0) - (c.z / 200.0));
	return vec3(x, y, z);
}

vec3 lchToLab(vec3 c) {
    return vec3(c.x, c.y * cos(c.z), c.y * sin(c.z));
}

vec4 waveColor(float lum, float chrom) {
    return vec4(rgbToSrgb(xyzToRgb(labToXyz(lchToLab(vec3(lum, chrom, time + 2 * screen_pos))))), 1.0);
}

vec4 colorBackground() {
    return vec4(0.0);
}

vec4 colorButton(bool pressed) {
    return waveColor(80.0, 100.0);
}

vec4 colorStick(vec2 stick_pos) {
    return waveColor(80.0, 100.0);
}

vec4 colorTrigger(float fill, bool pressed) {
    return waveColor(80.0, 100.0);
}
