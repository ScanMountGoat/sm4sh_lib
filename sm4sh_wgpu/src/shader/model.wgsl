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

// PerModel values
@group(1) @binding(0)
var<storage> skinning_transforms: array<mat4x4<f32>>;

@group(1) @binding(1)
var<storage> skinning_transforms_inv_transpose: array<mat4x4<f32>>;

// PerMaterial values
struct Uniforms {
    has_normal_map: u32,
    has_reflection_map: u32,
    has_reflection_cube_map: u32,
    // NU_ parameters
    ao_min_gain: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> uniforms: Uniforms;

// colorSampler in shaders.
@group(2) @binding(1)
var color_texture: texture_2d<f32>;

@group(2) @binding(2)
var color_sampler: sampler;

// normalSampler in shaders.
@group(2) @binding(3)
var normal_texture: texture_2d<f32>;

@group(2) @binding(4)
var normal_sampler: sampler;

// reflectionSampler in shaders.
@group(2) @binding(5)
var reflection_texture: texture_2d<f32>;

@group(2) @binding(6)
var reflection_sampler: sampler;

// reflectionCubeSampler in shaders.
@group(2) @binding(7)
var reflection_cube_texture: texture_cube<f32>;

@group(2) @binding(8)
var reflection_cube_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) color: vec4<f32>,
    @location(5) uv0: vec2<f32>,
}

struct PerMesh {
    parent_bone: vec4<i32>
}

@group(3) @binding(0)
var<uniform> per_mesh: PerMesh;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

// Define all possible attributes even if unused.
// This avoids needing separate shaders.
struct VertexInput0 {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) tangent: vec4<f32>,
    @location(3) bitangent: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) indices: vec4<u32>,
    @location(6) weights: vec4<f32>,
    @location(7) uv0: vec4<f32>,
}

@vertex
fn vs_main(in0: VertexInput0) -> VertexOutput {
    var out: VertexOutput;

    var position = vec3(0.0);
    var tangent = vec3(0.0);
    var normal = vec3(0.0);
    var bitangent = vec3(0.0);
    if per_mesh.parent_bone.x != -1 {
        let bone_index = per_mesh.parent_bone.x;

        position = (skinning_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
        tangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
        bitangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.bitangent.xyz, 0.0)).xyz;
        normal = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
    } else {
        for (var i = 0u; i < 4u; i += 1u) {
            let bone_index = in0.indices[i];
            let skin_weight = in0.weights[i];

            position += skin_weight * (skinning_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent += skin_weight * (skinning_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            bitangent += skin_weight * (skinning_transforms_inv_transpose[bone_index] * vec4(in0.bitangent.xyz, 0.0)).xyz;
            normal += skin_weight * (skinning_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        }
    }

    out.clip_position = camera.view_projection * vec4(position, 1.0);

    out.position = in0.position.xyz;
    out.normal = (camera.view * vec4(normal, 0.0)).xyz;
    out.tangent = (camera.view * vec4(tangent, 0.0)).xyz;
    out.bitangent = (camera.view * vec4(bitangent, 0.0)).xyz;
    out.color = in0.color;
    out.uv0 = in0.uv0.xy;
    return out;
}

// TODO: Port actual code from in game.
fn apply_normal_map(normal_map: vec3<f32>, tangent: vec3<f32>, bitangent: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Normal mapping is a change of basis using the TBN vectors.
    let x = normal_map.x;
    let y = normal_map.y;
    let z = normal_map.z;
    return normalize(tangent * x + bitangent * y + normal * z);
}

// Translated from Link's face fragment shader in RenderDoc with Cemu.
fn diffuse_ao_blend(ao: f32, ao_min_gain: vec4<f32>) -> vec3<f32> {
    // Calculate the effect of NU_aoMinGain on the ambient occlusion map.
    return clamp((1.0 - ao_min_gain.rgb) * ao + ao_min_gain.rgb, vec3(0.0), vec3(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // Normals are in view space, so the view vector is simple.
    let view = vec3(0.0, 0.0, 1.0);

    let normal_map_ao = textureSample(normal_texture, normal_sampler, in.uv0).rgba;
    let normal_map = normal_map_ao.rgb;
    var ao = 1.0;
    let vertex_tangent = normalize(in.tangent);
    let vertex_bitangent = normalize(in.bitangent);
    let vertex_normal = normalize(in.normal);
    // TODO: How is gamma handled for in game shaders?
    var normal = vertex_normal;
    if uniforms.has_normal_map == 1u {
        normal = apply_normal_map(pow(normal_map, vec3(2.2)), vertex_tangent, vertex_bitangent, vertex_normal);
        ao = normal_map_ao.a;
    }

    let lighting = max(dot(normal, view), 0.0);

    let color = textureSample(color_texture, color_sampler, in.uv0).rgba;
    let vertex_color = in.color * 2.0;

    let ao_blend = diffuse_ao_blend(ao, uniforms.ao_min_gain);

    var out_color = color.rgb * vertex_color.rgb * lighting * ao_blend;

    if uniforms.has_reflection_map == 1u {
        let sphere_uvs = vec2(normal.x * 0.5 + 0.5, 1.0 - (normal.y * 0.5 + 0.5));
        out_color += textureSample(reflection_texture, reflection_sampler, sphere_uvs).rgb;
    }

    let out_alpha = color.a * vertex_color.a;

    var out: FragmentOutput;
    out.color = vec4(out_color, out_alpha);
    return out;
}