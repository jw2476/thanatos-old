#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;

layout(set = 0, binding = 0) uniform Camera {
    mat4 viewProj;
} camera;

void main() {
    gl_Position = camera.viewProj * vec4(inPosition, 1.0);
    fragColor = inColor;
}
