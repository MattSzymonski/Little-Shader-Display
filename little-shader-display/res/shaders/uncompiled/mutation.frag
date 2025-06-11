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

// Sphere shape SDF
float signed_distance_sphere(vec3 position, float radius) {
    return length(position) - radius;
}

// Rounded box shape SDF
float signed_distance_round_box(vec3 position, vec3 boxSize, float radius) {
    vec3 distance_vector = abs(position) - boxSize;
    return length(max(distance_vector, 0.0)) + min(max(distance_vector.x, max(distance_vector.y, distance_vector.z)), 0.0) - radius;
}

// Smooth union of two SDFs
float smax(float a, float b, float k) {
    float h = max(k - abs(a - b), 0.0) / k;
    return max(a, b) + h * h * k * 0.25;
}

// Bubble-like noise effect using sin functions
float bubble_noise(vec3 position, float time) {
    float noise_size = 5.0;
    return sin(position.x * noise_size + time * 3.0) * 
           sin(position.y * noise_size + time * 3.5) * 
           sin(position.z * noise_size + time * 4.0) * 0.1;
}

// Composes the scene's SDF
float signed_distance_scene(vec3 position, float time) {
    vec3 cube_size = vec3(0.5);
    float cube_bevel = 0.20;
    float sphere_radius = ((sin(time) + 1.0) / 2.0) / 5.0 + 0.15;

    float box_distance = signed_distance_round_box(position, cube_size, cube_bevel);
    float sphere_distance = signed_distance_sphere(position, sphere_radius);

    float t = 0.5 + 0.5 * sin(time);
    float mutation = mix(box_distance, sphere_distance, t);

    box_distance += bubble_noise(position, time);
    return box_distance;
}

// Estimates surface normal using SDF gradient
vec3 compute_scene_normal(vec3 position, float time) {
    vec2 e = vec2(1e-4, 0);
    return normalize(vec3(
        signed_distance_scene(position + e.xyy, time) - signed_distance_scene(position, time),
        signed_distance_scene(position + e.yxy, time) - signed_distance_scene(position, time),
        signed_distance_scene(position + e.yyx, time) - signed_distance_scene(position, time)
    ));
}

void main() {
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;

    uv.x *= screen_aspect_ratio;

    // Setup camera
    vec3 camera_position = vec3(1.0, 1.0, 1.0);
    vec3 camera_target = vec3(0.0);
    vec3 forward = normalize(camera_target - camera_position);
    vec3 right = normalize(cross(vec3(0.0, 1.0, 0.0), forward));
    vec3 up = cross(forward, right);

    vec3 camera_direction = normalize(vec3(uv, 1.0));
    camera_direction = mat3(right, up, forward) * camera_direction;

    // Apply rotation to simulate orbiting camera
    float speedX = 1.0;
    float speedY = 1.0;

    float cX = cos(-time * speedX);
    float sX = sin(-time * speedX);
    float cY = cos(-time * speedY);
    float sY = sin(-time * speedY);

    camera_position.yz = vec2(cX * camera_position.y - sX * camera_position.z, sX * camera_position.y + cX * camera_position.z);
    camera_direction.yz = vec2(cX * camera_direction.y - sX * camera_direction.z, sX * camera_direction.y + cX * camera_direction.z);

    camera_position.xz = vec2(cY * camera_position.x - sY * camera_position.z, sY * camera_position.x + cY * camera_position.z);
    camera_direction.xz = vec2(cY * camera_direction.x - sY * camera_direction.z, sY * camera_direction.x + cY * camera_direction.z);

    // Raymarching loop
    float travel_distance = 0.0;
    float max_distance = 10.0;
    const int max_steps = 100;

    for (int i = 0; i < max_steps; i++) {
        vec3 currentPosition = camera_position + travel_distance * camera_direction;
        float distance = signed_distance_scene(currentPosition, time);

        if (abs(distance) < 1e-4 || travel_distance > max_distance) {
            break;
        }

        travel_distance += distance;
    }

    vec3 hit_position = camera_position + camera_direction * travel_distance;
    vec3 final_color = vec3(0.0); // Default background

    if (travel_distance < max_distance) {
        vec3 normal = compute_scene_normal(hit_position, time);
        vec3 lightDir = normalize(vec3(1.0, 2.0, 1.5));

        float diffuse = max(dot(normal, lightDir), 0.0);
        vec3 ambient = vec3(0.1) * 1.5;
        vec3 lighting = vec3(diffuse) + ambient;

        // Fake iridescence effect
        float interference = sin(dot(normal, camera_direction) * 15.0 + time * 2.0) * 0.6 + 0.5;
        vec3 iridescence = mix(vec3(0.1, 0.3, 0.4), vec3((sin(time * 5.0) + 1.0) / 2.0, 0.0, 0.1), interference);
        iridescence = mix(iridescence, vec3(0.5, 1.0, 0.7), interference * 0.5);

        float fresnel = mix(0.01, 0.4, pow(clamp(1.0 + dot(camera_direction, normal), 0.0, 1.0), 1.5));

        final_color = mix(lighting, iridescence, 0.3) + fresnel;
    }

    out_final_color = vec4(final_color, 1.0);
}
