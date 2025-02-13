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

// PerMaterial values
@group(1) @binding(0)
var color_texture: texture_2d<f32>;

@group(1) @binding(1)
var color_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

// Define all possible attributes even if unused.
// This avoids needing separate shaders.
struct VertexInput0 {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
}

@vertex
fn vs_main(in0: VertexInput0) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_projection * vec4(in0.position, 1.0);
    out.position = in0.position;
    out.normal = (camera.view * vec4(in0.normal, 0.0)).xyz;
    out.uv0 = in0.uv0;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);

    let lighting = dot(normalize(in.normal), view) * 0.5 + 0.5;

    let color = textureSample(color_texture, color_sampler, in.uv0).rgba;

    var out: FragmentOutput;
    out.color = color * lighting;
    return out;
}