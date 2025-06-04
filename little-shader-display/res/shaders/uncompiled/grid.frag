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

// Constants
const float pi = 3.141592;

// Main fragment shader
void main() {
    // Convert UV to [-1, 1] space centered around screen
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;

    uv.x *= screen_aspect_ratio;

    // Apply bluetooth_data.xy as an offset to the center position
    vec2 center = bluetooth_data.xy;

    // Distance from center
    float distance = length(uv - center);

    // Fading ripple effect
    float fade = exp(-5.5 * clamp(pow(distance, 3.0), 0.0, 1.0));

    // UV distortion based on ripple animation
    uv += uv * sin(pi * (time - distance)) * 0.3 * fade;

    // Grid parameters
    float stripes = 15.0;
    float thickness = 10.0;

    // Create animated sine stripes
    vec2 a = sin(stripes * 0.5 * pi * uv - pi / 2.0);
    vec2 b = abs(a);

    // Compose layered grid effect
    vec3 color = vec3(0.0);
    color += 1.0 * exp(-thickness * b.x * (0.8 + 0.5 * sin(pi * time))); // Horizontal lines
    color += 1.0 * exp(-thickness * b.y);                                // Vertical lines
    color += 0.5 * exp(-(thickness / 4.0) * sin(b.x));                   // Horizontal soft glow
    color += 0.5 * exp(-(thickness / 3.0) * b.y);                        // Vertical soft glow

    // Color blending (ripple red, grid white)
    vec3 rippleColor = vec3(1.0, 0.0, 0.0);
    vec3 gridColor = vec3(1.0);
    vec3 finalColor = mix(gridColor, rippleColor, fade) * color;

    // Output final fragment color
    out_final_color = vec4(finalColor, 1.0);
}
