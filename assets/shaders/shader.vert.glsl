#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0) uniform Camera {
    mat4 viewProj;
} camera;

void main() {
    gl_Position = camera.viewProj * vec4(position, 1.0);
    fragColor = normal * 0.5 + 0.5;
}
