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

void main() {
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;
    uv.x *= screen_aspect_ratio;

    float sinePosition = 0.725 * sin(uv.x * 4.0 - time * 4.0) * cos(uv.x * 8.0 - time * 3.0) * 0.5;
    vec3 color_top = vec3(step(uv.y, sinePosition));
    out_final_color = vec4(color_top, 1.0); 
}