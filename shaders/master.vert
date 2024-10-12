#version 450

// Input vertex data
layout(location=0) in vec2 vertex_position;
layout(location=1) in vec2 vertex_texture_coordinates;

// Output data
layout(location=0) out vec2 out_vertex_position;
layout(location=1) out vec2 out_vertex_texture_coordinates;

void main() {
    out_vertex_position = vertex_position;
    out_vertex_texture_coordinates = vertex_texture_coordinates;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
}