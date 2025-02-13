// PerScene values.
struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_projection: mat4x4<f32>,
    position: vec4<f32>,
    resolution: vec2<f32>
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

// Define all possible attributes even if unused.
// This avoids needing separate shaders.
struct VertexInput0 {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

@vertex
fn vs_main(in0: VertexInput0) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_projection * vec4(in0.position, 1.0);
    out.position = in0.position;
    out.normal = in0.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4(normalize(in.normal) * 0.5 + 0.5, 1.0);
    return out;
}