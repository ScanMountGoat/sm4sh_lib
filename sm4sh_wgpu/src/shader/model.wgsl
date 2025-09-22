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

// TODO: Use snake_case
// FB0 in shaders.
struct Fb0 {
    depthOfField0: vec4<f32>,
    depthOfField1: vec4<f32>,
    depthOfFieldTexSize: vec4<f32>,
    projInvMatrix: mat4x4<f32>,
    refraction_param: vec4<f32>,
    proj_to_view: vec4<f32>,
    view_to_proj: vec4<f32>,
    gi_buffer_size: vec4<f32>,
    weight0: vec4<f32>,
    weight1: vec4<f32>,
    random_vector: array<vec4<f32>, 31>,
    reflection_param: vec4<f32>,
    sun_shaft_light_param0: array<vec4<f32>, 2>,
    sun_shaft_light_param1: array<vec4<f32>, 2>,
    sun_shaft_blur_param: array<vec4<f32>, 4>,
    sun_shaft_composite_param: array<vec4<f32>, 2>,
    glare_abstract_param: vec4<f32>,
    renderTargetTexSize: vec4<f32>,
    glare_fog_param: array<vec4<f32>, 2>,
    glare_simple_color: vec4<f32>,
    pad0_FB0: vec4<f32>,
    lens_flare_param: vec4<f32>,
    outline_param: vec4<f32>,
    post_reflection_color: vec4<f32>,
    MultiShadowMatrix: array<mat4x4<f32>, 4>,
    ShadowMapMatrix: mat4x4<f32>,
    view: mat4x4<f32>,
    eye: vec4<f32>,
    constantColor: vec4<f32>,
    lightMapPos: vec4<f32>,
    reflectionGain: vec4<f32>,
    hdrConstant: vec4<f32>,
    _g_fresnelColor: vec4<f32>,
    effect_light_param0: vec4<f32>,
    effect_light_param1: vec4<f32>,
    bgRotInv: mat4x4<f32>,
    reflectionColor1: vec4<f32>,
    reflectionColor2: vec4<f32>,
    reflectionColor3: vec4<f32>,
    effect_light_param2: vec4<f32>,
}

@group(0) @binding(1)
var<uniform> fb0: Fb0;

// FB1 in shaders.
struct Fb1 {
    lightMapMatrix: mat4x4<f32>,
    blinkColor: vec4<f32>,
    g_constantVolume: vec4<f32>,
    g_constantOffset: vec4<f32>,
    uvScrollCounter: vec4<f32>,
    spycloakParams: vec4<f32>,
    compressParam: vec4<f32>,
    g_fresnelColor: vec4<f32>,
    depthOffset: vec4<f32>,
    outlineColor: vec4<f32>,
    pad0_FB1: array<vec4<f32>, 3>,
    lightMapColorGain: vec4<f32>,
    lightMapColorOffset: vec4<f32>,
    ceilingDir: vec4<f32>,
    ceilingColor: vec4<f32>,
    groundColor: vec4<f32>,
    ambientColor: vec4<f32>,
    lightDirColor1: vec4<f32>,
    lightDirColor2: vec4<f32>,
    lightDirColor3: vec4<f32>,
    lightDir1: vec4<f32>,
    lightDir2: vec4<f32>,
    lightDir3: vec4<f32>,
    fogColor: vec4<f32>,
    g_fresnelOffset: vec4<f32>,
    ShadowMapParam: vec4<f32>,
    charShadowColor: vec4<f32>,
    charShadowColor2: vec4<f32>,
    softLightingParams2: vec4<f32>,
    bgShadowColor: vec4<f32>,
    g_iblColorGain: vec4<f32>,
    g_iblColorOffset: vec4<f32>,
    g_constantMin: vec4<f32>,
    loupeShadowParams: vec4<f32>,
    softLightColorGain: vec4<f32>,
    softLightColorOffset: vec4<f32>,
    characterColor: vec4<f32>,
}

@group(0) @binding(2)
var<uniform> fb1: Fb1;

// FB3 in shaders.
struct Fb3 {
    hdrRange: vec4<f32>,
    colrHdrRange: vec4<f32>
}

@group(0) @binding(3)
var<uniform> fb3: Fb3;

// FB4 in shaders.
struct Fb4 {
    effect_light_entry: vec4<f32>
}

@group(0) @binding(4)
var<uniform> fb4: Fb4;

// FB5 in shaders.
struct Fb5 {
    effect_light_area: vec4<u32>
}

@group(0) @binding(5)
var<uniform> fb5: Fb5;

// PerModel values
@group(1) @binding(0)
var<storage> skinning_transforms: array<mat4x4<f32>>;

@group(1) @binding(1)
var<storage> skinning_transforms_inv_transpose: array<mat4x4<f32>>;


// MC in shaders with only the used parameters.
struct Uniforms {
    // NU_ parameters
    alphaBlendParams: vec4<f32>,
    angleFadeParams: vec4<f32>,
    aoMinGain: vec4<f32>,
    colorGain: vec4<f32>,
    colorOffset: vec4<f32>,
    colorSampler2UV: vec4<f32>,
    colorSampler3UV: vec4<f32>,
    colorSampler4UV: vec4<f32>,
    colorSamplerUV: vec4<f32>,
    colorStepUV: vec4<f32>,
    customSoftLightParams: vec4<f32>,
    diffuseColor: vec4<f32>,
    dualNormalScrollParams: vec4<f32>,
    finalColorGain: vec4<f32>,
    finalColorGain2: vec4<f32>,
    finalColorGain3: vec4<f32>,
    fogParams: vec4<f32>,
    fresnelColor: vec4<f32>,
    fresnelParams: vec4<f32>,
    normalParams: vec4<f32>,
    normalSamplerAUV: vec4<f32>,
    normalSamplerBUV: vec4<f32>,
    reflectionColor: vec4<f32>,
    reflectionParams: vec4<f32>,
    rotatePivotUV: vec4<f32>,
    softLightingParams: vec4<f32>,
    specularColor: vec4<f32>,
    specularColorGain: vec4<f32>,
    specularParams: vec4<f32>,
    testParam0: vec4<f32>,
    testParam1: vec4<f32>,
    testParam2: vec4<f32>,
    testParam3: vec4<f32>,
    translucentColor: vec4<f32>,
    zOffset: vec4<f32>,
}

@group(2) @binding(5)
var<uniform> uniforms: Uniforms;

// MC_EFFECT in shaders with only the used parameters.
struct EffectUniforms {
    angleFadeParams: vec4<f32>,
    effColorGain: vec4<f32>,
    effCombinerAlpha0: vec4<f32>,
    effCombinerColor0: vec4<f32>,
    effCombinerColor1: vec4<f32>,
    effDepthOffset: vec4<f32>,
    effMTBlendAlpha: vec4<f32>,
    effMTBlendParam0: vec4<f32>,
    effMTBlendParam1: vec4<f32>,
    effMTBlendParam2: vec4<f32>,
    effRefractParam: vec4<f32>,
    effRotUV: vec4<f32>,
    effScaleUV: vec4<f32>,
    effSilhouetteColor: vec4<f32>,
    effSunShaftParams0: vec4<f32>,
    effSunShaftParams1: vec4<f32>,
    effTransUV: vec4<f32>,
    effUniverseParam: vec4<f32>,
    effYGradColorBottom: vec4<f32>,
    effYGradColorTop: vec4<f32>,
    effYGradParam: vec4<f32>,
    normalParams: vec4<f32>,
    normalSamplerAUV: vec4<f32>,
}

@group(2) @binding(6)
var<uniform> effect_uniforms: EffectUniforms;

// colorSampler in shaders.
@group(2) @binding(7)
var color_texture: texture_2d<f32>;

@group(2) @binding(8)
var color_sampler: sampler;

// normalSampler in shaders.
@group(2) @binding(9)
var normal_texture: texture_2d<f32>;

@group(2) @binding(10)
var normal_sampler: sampler;

// reflectionSampler in shaders.
@group(2) @binding(11)
var reflection_texture: texture_2d<f32>;

@group(2) @binding(12)
var reflection_sampler: sampler;

// reflectionCubeSampler in shaders.
@group(2) @binding(13)
var reflection_cube_texture: texture_cube<f32>;

@group(2) @binding(14)
var reflection_cube_sampler: sampler;

// color2Sampler in shaders.
@group(2) @binding(15)
var color2_texture: texture_2d<f32>;

@group(2) @binding(16)
var color2_sampler: sampler;

// diffuseSampler in shaders.
@group(2) @binding(17)
var diffuse_texture: texture_2d<f32>;

@group(2) @binding(18)
var diffuse_sampler: sampler;

// lightMapSampler in shaders.
@group(2) @binding(19)
var light_map_texture: texture_2d<f32>;

@group(2) @binding(20)
var light_map_sampler: sampler;

// normalSampler in shaders.
@group(2) @binding(21)
var normal2_texture: texture_2d<f32>;

@group(2) @binding(22)
var normal2_sampler: sampler;

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
    parent_bone: i32,
    has_skinning: u32,
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

    var position = in0.position.xyz;
    var tangent = in0.tangent.xyz;
    var normal = in0.normal.xyz;
    var bitangent = in0.bitangent.xyz;
    if per_mesh.parent_bone != -1 {
        let bone_index = per_mesh.parent_bone;

        position = (skinning_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
        tangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
        bitangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.bitangent.xyz, 0.0)).xyz;
        normal = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
    } else if per_mesh.has_skinning == 1u {
        position = vec3(0.0);
        tangent = vec3(0.0);
        normal = vec3(0.0);
        bitangent = vec3(0.0);

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

    out.position = position.xyz;
    out.normal = normal;
    out.tangent = tangent;
    out.bitangent = bitangent;
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
    // Required for wgsl_to_wgpu reachability analysis to include these resources.
    let REMOVE_BEGIN = 0.0;
    var _unused = textureSample(color_texture, color_sampler, vec2(0.0));
    _unused = textureSample(normal_texture, normal_sampler, vec2(0.0));
    _unused = textureSample(reflection_texture, reflection_sampler, vec2(0.0));
    _unused = textureSample(reflection_cube_texture, reflection_cube_sampler, vec3(0.0));
    _unused = textureSample(color2_texture, color2_sampler, vec2(0.0));
    _unused = textureSample(diffuse_texture, diffuse_sampler, vec2(0.0));
    _unused = textureSample(light_map_texture, light_map_sampler, vec2(0.0));
    _unused = textureSample(normal2_texture, normal2_sampler, vec2(0.0));
    _unused = uniforms.aoMinGain;
    _unused = effect_uniforms.angleFadeParams;
    _unused = fb0.lens_flare_param;
    _unused = fb1.ShadowMapParam;
    _unused = fb3.colrHdrRange;
    _unused = fb4.effect_light_entry;
    _unused = vec4<f32>(fb5.effect_light_area);
    _unused = camera.projection[0];
    let REMOVE_END = 0.0;

    let vertex_tangent = normalize(in.tangent);
    let vertex_bitangent = normalize(in.bitangent);
    let vertex_normal = normalize(in.normal);
    var normal = vertex_normal;

    // TODO: Rename these in the shadergen itself?
    let a_Position = vec4(in.position, 0.0);
    let a_TexCoord0 = vec4(in.uv0, 0.0, 0.0);
    let a_TexCoord1 = vec4(0.0);
    let a_TexCoord2 = vec4(0.0);
    let a_Normal = vec4(vertex_normal, 0.0);
    let a_Tangent = vec4(vertex_tangent, 0.0);
    let a_Binormal = vec4(vertex_bitangent, 0.0);
    let a_Color = in.color;

    // TODO: Figure out how to initialize this.
    let local_to_world_matrix = mat4x4(
        vec4(0.0, 0.0, 1.0, 0.0),
        vec4(0.0, 1.0, 0.0, 0.0),
        vec4(1.0, 0.0, 0.0, 0.0),
        vec4(59.99999, 0.01, 0.0, 1.0)
    );

    var out_color = vec4(0.0);

    // Replaced with generated code.
    let ASSIGN_VARS_GENERATED = 0.0;
    let ASSIGN_OUT_COLOR_GENERATED = 0.0;

    // TODO: How is gamma handled for in game shaders?
    var out: FragmentOutput;
    out.color = out_color;
    return out;
}