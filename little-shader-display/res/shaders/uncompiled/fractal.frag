#version 450

// Input vertex data
layout(location = 0) in vec2 vertex_position;
layout(location = 1) in vec2 vertex_texture_coordinates;

// Uniforms
layout(set = 0, binding = 0) uniform Uniforms {
    float time;
    vec3 bluetooth_data;
    float screen_aspect_ratio;
};

// Output fragment color
layout(location = 0) out vec4 out_final_color;

// -------- Noise functions --------
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
        mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
        u.y
    );
}

float fbm(vec2 p) {
    float total = 0.0;
    float amplitude = 0.6;
    for (int i = 0; i < 4; i++) {
        total += amplitude * noise(p);
        p *= 2.0;
        amplitude *= 0.5;
    }
    return total;
}

vec3 posterize(vec3 color, float levels) {
    return floor(color * levels) / levels;
}

void main() {
    vec2 uv = vertex_texture_coordinates;
    uv -= 0.5;
    uv.x *= screen_aspect_ratio;

    float r = length(uv) + 0.05;
    float a = atan(uv.y, uv.x);

    // Rotational flow over time
    a += time * 0.1;

    float symmetry = 8.0;
    a = mod(a, 6.2831 / symmetry);
    a = abs(a - 3.14159 / symmetry);

    vec2 kaleido_uv = vec2(cos(a), sin(a)) * r;

    // Apply tilt distortion and swirl
    kaleido_uv += bluetooth_data.xy * 0.6;
    kaleido_uv += vec2(
        sin(r * 10.0 - time * 2.0),
        cos(r * 10.0 - time * 2.0)
    ) * 0.02;

    // Radial pulse modulation
    float pulse = 0.3 + 0.25 * sin(time * 2.0 + r * 20.0);
    float pattern = fbm(kaleido_uv * 8.0 + pulse);

    // Colorful, vibrant palette with less pink
    vec3 color = vec3(
        0.6 + 0.6 * cos(6.2831 * pattern + 0.5),  // Red: warm & dominant
        0.5 + 0.4 * cos(6.2831 * pattern + 0.6),  // Green: synced with red for yellow
        0.3 + 0.3 * cos(6.2831 * pattern + 2.5)   // Blue: reduced for less purple/pink
    ) * 1.3;

    // Posterize for stylization
    color = posterize(color, 3.0);

    // High-frequency shimmer/glow
    float glow = smoothstep(0.15, 0.35, abs(dFdx(pattern)) + abs(dFdy(pattern)));
    color += glow * 0.3;

    // Soft vignette to draw focus inward
    float vignette = smoothstep(0.6, 0.5, r);
    color *= vignette;

    // Optional edge outlining
    float edge = smoothstep(0.01, 0.015, abs(dFdx(pattern)) + abs(dFdy(pattern)));
    color *= 1.0 - edge * 0.5;

    out_final_color = vec4(color, 1.0);
}
