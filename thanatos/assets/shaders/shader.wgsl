struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec3<f32>
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec3<f32>,
}

struct Camera {
    view_proj: mat4x4<f32>
}
@group(0) @binding(0) var<uniform> camera: Camera;

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * vec4<f32>(vertex.position, 1.0);
    out.normal = vertex.normal;
    out.colour = vertex.colour;
    return out;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(vertex.colour * (0.5 + 0.25 * dot(vertex.normal, vec3<f32>(1.0))), 1.0);
}
