#version 450

// Input vertex data
layout(location = 0) in vec2 vertex_position;
layout(location = 1) in vec2 vertex_texture_coordinates;

layout(set = 0, binding = 0) uniform Uniforms {
    float time;
};

// Output data
layout(location = 0) out vec4 out_final_color;

void main() {
    // Normalize texture coordinates to [-1, 1]
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;

    // Compute distance from center
    float dist = length(uv);

    // Pulsate radius using sine wave
    float radius = 0.3 + 0.1 * sin(time * 2.0);

    // Smooth edge transition
    float edge = 0.01;
    float circle = smoothstep(radius + edge, radius - edge, dist);

    // Mix red inside the circle, black outside
    vec3 color = mix(vec3(0.0), vec3(1.0, 0.0, 0.0), circle);

    // Always fully opaque
    out_final_color = vec4(color, 1.0);
}
