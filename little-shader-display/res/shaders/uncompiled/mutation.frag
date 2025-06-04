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
float signedDistanceSphere(vec3 position, float radius) {
    return length(position) - radius;
}

// Rounded box shape SDF
float signedDistanceRoundBox(vec3 position, vec3 boxSize, float radius) {
    vec3 distanceVector = abs(position) - boxSize;
    return length(max(distanceVector, 0.0)) + min(max(distanceVector.x, max(distanceVector.y, distanceVector.z)), 0.0) - radius;
}

// Smooth union of two SDFs
float smax(float a, float b, float k) {
    float h = max(k - abs(a - b), 0.0) / k;
    return max(a, b) + h * h * k * 0.25;
}

// Bubble-like noise effect using sin functions
float bubbleNoise(vec3 position, float time) {
    float noiseSize = 5.0;
    return sin(position.x * noiseSize + time * 3.0) * 
           sin(position.y * noiseSize + time * 3.5) * 
           sin(position.z * noiseSize + time * 4.0) * 0.1;
}

// Composes the scene's SDF
float signedDistanceScene(vec3 position, float time) {
    vec3 cubeSize = vec3(0.5);
    float cubeBevel = 0.20;
    float sphereRadius = ((sin(time) + 1.0) / 2.0) / 5.0 + 0.15;

    float boxDist = signedDistanceRoundBox(position, cubeSize, cubeBevel);
    float sphereDist = signedDistanceSphere(position, sphereRadius);

    float t = 0.5 + 0.5 * sin(time);
    float mutation = mix(boxDist, sphereDist, t);

    boxDist += bubbleNoise(position, time);
    return boxDist;
}

// Estimates surface normal using SDF gradient
vec3 computeSceneNormal(vec3 position, float time) {
    vec2 e = vec2(1e-4, 0);
    return normalize(vec3(
        signedDistanceScene(position + e.xyy, time) - signedDistanceScene(position, time),
        signedDistanceScene(position + e.yxy, time) - signedDistanceScene(position, time),
        signedDistanceScene(position + e.yyx, time) - signedDistanceScene(position, time)
    ));
}

void main() {
    vec2 uv = vertex_texture_coordinates * 2.0 - 1.0;

    uv.x *= screen_aspect_ratio;

    // Setup camera
    vec3 cameraPosition = vec3(1.0, 1.0, 1.0);
    vec3 cameraTarget = vec3(0.0);
    vec3 forward = normalize(cameraTarget - cameraPosition);
    vec3 right = normalize(cross(vec3(0.0, 1.0, 0.0), forward));
    vec3 up = cross(forward, right);

    vec3 cameraDirection = normalize(vec3(uv, 1.0));
    cameraDirection = mat3(right, up, forward) * cameraDirection;

    // Apply rotation to simulate orbiting camera
    float speedX = 1.0;
    float speedY = 1.0;

    float cX = cos(-time * speedX);
    float sX = sin(-time * speedX);
    float cY = cos(-time * speedY);
    float sY = sin(-time * speedY);

    cameraPosition.yz = vec2(cX * cameraPosition.y - sX * cameraPosition.z, sX * cameraPosition.y + cX * cameraPosition.z);
    cameraDirection.yz = vec2(cX * cameraDirection.y - sX * cameraDirection.z, sX * cameraDirection.y + cX * cameraDirection.z);

    cameraPosition.xz = vec2(cY * cameraPosition.x - sY * cameraPosition.z, sY * cameraPosition.x + cY * cameraPosition.z);
    cameraDirection.xz = vec2(cY * cameraDirection.x - sY * cameraDirection.z, sY * cameraDirection.x + cY * cameraDirection.z);

    // Raymarching loop
    float travelDistance = 0.0;
    float maxDistance = 10.0;
    const int maxSteps = 100;

    for (int i = 0; i < maxSteps; i++) {
        vec3 currentPosition = cameraPosition + travelDistance * cameraDirection;
        float distance = signedDistanceScene(currentPosition, time);

        if (abs(distance) < 1e-4 || travelDistance > maxDistance) {
            break;
        }

        travelDistance += distance;
    }

    vec3 hitPosition = cameraPosition + cameraDirection * travelDistance;
    vec3 finalColor = vec3(0.0); // Default background

    if (travelDistance < maxDistance) {
        vec3 normal = computeSceneNormal(hitPosition, time);
        vec3 lightDir = normalize(vec3(1.0, 2.0, 1.5));

        float diffuse = max(dot(normal, lightDir), 0.0);
        vec3 ambient = vec3(0.1) * 1.5;
        vec3 lighting = vec3(diffuse) + ambient;

        // Fake iridescence effect
        float interference = sin(dot(normal, cameraDirection) * 15.0 + time * 2.0) * 0.6 + 0.5;
        vec3 iridescence = mix(vec3(0.1, 0.3, 0.4), vec3((sin(time * 5.0) + 1.0) / 2.0, 0.0, 0.1), interference);
        iridescence = mix(iridescence, vec3(0.5, 1.0, 0.7), interference * 0.5);

        float fresnel = mix(0.01, 0.4, pow(clamp(1.0 + dot(cameraDirection, normal), 0.0, 1.0), 1.5));

        finalColor = mix(lighting, iridescence, 0.3) + fresnel;
    }

    out_final_color = vec4(finalColor, 1.0);
}
