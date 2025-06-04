#version 450

layout(location = 0) in vec2 vertex_position;
layout(location = 1) in vec2 vertex_texture_coordinates;

layout(set = 0, binding = 0) uniform Uniforms {
    float time;
    vec3 bluetooth_data; 
    float screen_aspect_ratio;
};

layout(location = 0) out vec4 out_final_color;

void main() {
    // Normalize UV coordinates to [-1, 1] range
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;
    uv.x *= screen_aspect_ratio;

    // Bluetooth tilt input, scaled
    vec2 tilt = bluetooth_data.xy * 0.5;

    // Compute polar coordinates from original UV
    float radius = length(uv);
    float angle = atan(uv.y, uv.x);

    // Center offset fades with radius (more shift near center, none at edge)
    float radius_factor = 1.0 - smoothstep(0.0, 1.0, radius);
    vec2 offset = tilt * radius_factor;
    vec2 warped_uv = uv - offset;

    float warped_radius = length(warped_uv);

    // Tunnel ring parameters
    float frequency = 12.0;
    float ring_width = 0.25;
    float edge_thickness = 0.04;
    float speed = 1.2;

    float phase = warped_radius * frequency - time * speed;
    float ring_wave = fract(phase);
    float ring = smoothstep(ring_width, ring_width - edge_thickness, ring_wave);

    // Optional radial fade
    float vignette = smoothstep(1.0, 0.2, radius);

    // Final color blending
    vec3 color = mix(vec3(0.0), vec3(1.0, 0.2, 0.2), ring * vignette);
    out_final_color = vec4(color, 1.0);
}
