#version 450

// Input vertex data
layout(location=0) in vec2 vertex_position;
layout(location=1) in vec2 vertex_texture_coordinates;

layout(set = 0, binding = 0) uniform Uniforms {
    float time;
};

// Output data
layout(location=0) out vec4 out_final_color;

void main() {
    float sinePosition = 0.5 + 0.325 * sin(vertex_texture_coordinates.x * 10.0 - time * 4.0)* cos(vertex_texture_coordinates.x * 50.0  - time * 3.0);
    vec3 colortop = vec3(step(vertex_texture_coordinates.y, sinePosition));
    out_final_color = vec4(colortop, 1.0); 
}