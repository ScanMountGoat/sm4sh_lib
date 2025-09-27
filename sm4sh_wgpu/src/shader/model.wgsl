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

// FB0 in shaders.
struct Fb0 {
    depth_of_field0: vec4<f32>,
    depth_of_field1: vec4<f32>,
    depth_of_field_tex_size: vec4<f32>,
    proj_inv_matrix: mat4x4<f32>,
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
    render_target_tex_size: vec4<f32>,
    glare_fog_param: array<vec4<f32>, 2>,
    glare_simple_color: vec4<f32>,
    pad0_fb0: vec4<f32>,
    lens_flare_param: vec4<f32>,
    outline_param: vec4<f32>,
    post_reflection_color: vec4<f32>,
    multi_shadow_matrix: array<mat4x4<f32>, 4>,
    shadow_map_matrix: mat4x4<f32>,
    view: mat4x4<f32>,
    eye: vec4<f32>,
    constant_color: vec4<f32>,
    light_map_pos: vec4<f32>,
    reflection_gain: vec4<f32>,
    hdr_constant: vec4<f32>,
    _g_fresnel_color: vec4<f32>,
    effect_light_param0: vec4<f32>,
    effect_light_param1: vec4<f32>,
    bg_rot_inv: mat4x4<f32>,
    reflection_color1: vec4<f32>,
    reflection_color2: vec4<f32>,
    reflection_color3: vec4<f32>,
    effect_light_param2: vec4<f32>,
}

@group(0) @binding(1)
var<uniform> fb0: Fb0;

// FB1 in shaders.
struct Fb1 {
    light_map_matrix: mat4x4<f32>,
    blink_color: vec4<f32>,
    g_constant_volume: vec4<f32>,
    g_constant_offset: vec4<f32>,
    uv_scroll_counter: vec4<f32>,
    spycloak_params: vec4<f32>,
    compress_param: vec4<f32>,
    g_fresnel_color: vec4<f32>,
    depth_offset: vec4<f32>,
    outline_color: vec4<f32>,
    pad0_fb1: array<vec4<f32>, 3>,
    light_map_color_gain: vec4<f32>,
    light_map_color_offset: vec4<f32>,
    ceiling_dir: vec4<f32>,
    ceiling_color: vec4<f32>,
    ground_color: vec4<f32>,
    ambient_color: vec4<f32>,
    light_dir_color1: vec4<f32>,
    light_dir_color2: vec4<f32>,
    light_dir_color3: vec4<f32>,
    light_dir1: vec4<f32>,
    light_dir2: vec4<f32>,
    light_dir3: vec4<f32>,
    fog_color: vec4<f32>,
    g_fresnel_offset: vec4<f32>,
    shadow_map_param: vec4<f32>,
    char_shadow_color: vec4<f32>,
    char_shadow_color2: vec4<f32>,
    soft_lighting_params2: vec4<f32>,
    bg_shadow_color: vec4<f32>,
    g_ibl_color_gain: vec4<f32>,
    g_ibl_color_offset: vec4<f32>,
    g_constant_min: vec4<f32>,
    loupe_shadow_params: vec4<f32>,
    soft_light_color_gain: vec4<f32>,
    soft_light_color_offset: vec4<f32>,
    character_color: vec4<f32>,
}

@group(0) @binding(2)
var<uniform> fb1: Fb1;

// FB3 in shaders.
struct Fb3 {
    hdr_range: vec4<f32>,
    colr_hdr_range: vec4<f32>
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

@group(1) @binding(2)
var<storage> bone_transforms: array<mat4x4<f32>>;

// MC in shaders with only the used parameters.
struct Uniforms {
    // NU_ parameters
    alpha_blend_params: vec4<f32>,
    angle_fade_params: vec4<f32>,
    ao_min_gain: vec4<f32>,
    color_gain: vec4<f32>,
    color_offset: vec4<f32>,
    color_sampler2_u_v: vec4<f32>,
    color_sampler3_u_v: vec4<f32>,
    color_sampler4_u_v: vec4<f32>,
    color_sampler_u_v: vec4<f32>,
    color_step_u_v: vec4<f32>,
    custom_soft_light_params: vec4<f32>,
    diffuse_color: vec4<f32>,
    dual_normal_scroll_params: vec4<f32>,
    final_color_gain: vec4<f32>,
    final_color_gain2: vec4<f32>,
    final_color_gain3: vec4<f32>,
    fog_params: vec4<f32>,
    fresnel_color: vec4<f32>,
    fresnel_params: vec4<f32>,
    normal_params: vec4<f32>,
    normal_sampler_a_u_v: vec4<f32>,
    normal_sampler_b_u_v: vec4<f32>,
    reflection_color: vec4<f32>,
    reflection_params: vec4<f32>,
    rotate_pivot_u_v: vec4<f32>,
    soft_lighting_params: vec4<f32>,
    specular_color: vec4<f32>,
    specular_color_gain: vec4<f32>,
    specular_params: vec4<f32>,
    test_param0: vec4<f32>,
    test_param1: vec4<f32>,
    test_param2: vec4<f32>,
    test_param3: vec4<f32>,
    translucent_color: vec4<f32>,
    z_offset: vec4<f32>,
}

@group(2) @binding(5)
var<uniform> uniforms: Uniforms;

// MC_EFFECT in shaders with only the used parameters.
struct EffectUniforms {
    angle_fade_params: vec4<f32>,
    eff_color_gain: vec4<f32>,
    eff_combiner_alpha0: vec4<f32>,
    eff_combiner_color0: vec4<f32>,
    eff_combiner_color1: vec4<f32>,
    eff_depth_offset: vec4<f32>,
    eff_m_t_blend_alpha: vec4<f32>,
    eff_m_t_blend_param0: vec4<f32>,
    eff_m_t_blend_param1: vec4<f32>,
    eff_m_t_blend_param2: vec4<f32>,
    eff_refract_param: vec4<f32>,
    eff_rot_u_v: vec4<f32>,
    eff_scale_u_v: vec4<f32>,
    eff_silhouette_color: vec4<f32>,
    eff_sun_shaft_params0: vec4<f32>,
    eff_sun_shaft_params1: vec4<f32>,
    eff_trans_u_v: vec4<f32>,
    eff_universe_param: vec4<f32>,
    eff_y_grad_color_bottom: vec4<f32>,
    eff_y_grad_color_top: vec4<f32>,
    eff_y_grad_param: vec4<f32>,
    normal_params: vec4<f32>,
    normal_sampler_a_u_v: vec4<f32>,
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
    @location(6) uv1: vec2<f32>,
    @location(7) uv2: vec2<f32>,
}

struct PerMesh {
    parent_bone: i32,
    has_skinning: u32,
    is_nsc: u32,
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
    @location(7) uv01: vec4<f32>,
    @location(8) uv23: vec4<f32>,
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

        if per_mesh.is_nsc == 1u {
            // Parenting with the parent transform.
            position = (bone_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent = (bone_transforms[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            bitangent = (bone_transforms[bone_index] * vec4(in0.bitangent.xyz, 0.0)).xyz;
            normal = (bone_transforms[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        } else {
            // Parenting that assumes the base parent transform is already applied.
            position = (skinning_transforms[bone_index] * vec4(in0.position.xyz, 1.0)).xyz;
            tangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.tangent.xyz, 0.0)).xyz;
            bitangent = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.bitangent.xyz, 0.0)).xyz;
            normal = (skinning_transforms_inv_transpose[bone_index] * vec4(in0.normal.xyz, 0.0)).xyz;
        }
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
    out.uv0 = in0.uv01.xy;
    out.uv1 = in0.uv01.zw;
    out.uv2 = in0.uv23.xy;
    return out;
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
    _unused = uniforms.ao_min_gain;
    _unused = effect_uniforms.angle_fade_params;
    _unused = fb0.lens_flare_param;
    _unused = fb1.shadow_map_param;
    _unused = fb3.colr_hdr_range;
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
    let a_TexCoord1 = vec4(in.uv1, 0.0, 0.0);
    let a_TexCoord2 = vec4(in.uv2, 0.0, 0.0);
    let a_Normal = vec4(vertex_normal, 0.0);
    let a_Tangent = vec4(vertex_tangent, 0.0);
    let a_Binormal = vec4(vertex_bitangent, 0.0);
    let a_Color = in.color;

    // TODO: Figure out how to initialize this.
    let local_to_world_matrix = mat4x4(
        vec4(1.0, 0.0, 0.0, 0.0),
        vec4(0.0, 1.0, 0.0, 0.0),
        vec4(0.0, 0.0, 1.0, 0.0),
        vec4(0.0, 0.0, 0.0, 1.0)
    );

    var out_color = vec4(0.0);

    // Replaced with generated code.
    let ASSIGN_VARS_GENERATED = 0.0;
    let ASSIGN_OUT_COLOR_GENERATED = 0.0;

    var out: FragmentOutput;
    out.color = out_color;

    let ALPHA_TEST_GENERATED = 0.0;

    return out;
}