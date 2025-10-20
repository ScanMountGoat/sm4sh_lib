use std::collections::BTreeMap;

use log::error;

use glam::{Vec4, vec4};
use sm4sh_model::NudMesh;

use crate::{DeviceBufferExt, SharedData};

pub fn create_bind_group2(
    device: &wgpu::Device,
    mesh: &NudMesh,
    hash_to_texture: &BTreeMap<u32, wgpu::TextureView>,
    default_texture: &wgpu::TextureView,
    default_cube_texture: &wgpu::TextureView,
    shared_data: &SharedData,
) -> crate::shader::model::bind_groups::BindGroup2 {
    // TODO: Load all textures and samplers.
    let mut color_texture = None;
    let mut color_sampler = None;

    let mut color2_texture = None;
    let mut color2_sampler = None;

    let mut normal_texture = None;
    let mut normal_sampler = None;

    let mut normal2_texture = None;
    let mut normal2_sampler = None;

    let mut reflection_texture = None;
    let mut reflection_sampler = None;

    let mut reflection_cube_texture = None;
    let mut reflection_cube_sampler = None;

    let mut diffuse_texture = None;
    let mut diffuse_sampler = None;

    let mut light_map_texture = None;
    let mut light_map_sampler = None;

    if let Some(material) = &mesh.material1 {
        if let Some(program) = shared_data.database.get_shader(material.shader_id) {
            for (s, texture) in program.samplers.iter().zip(&material.textures) {
                match s.as_str() {
                    "colorSampler" => {
                        color_texture = hash_to_texture.get(&texture.hash);
                        color_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "normalSampler" => {
                        normal_texture = hash_to_texture.get(&texture.hash);
                        normal_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "normal2Sampler" => {
                        normal2_texture = hash_to_texture.get(&texture.hash);
                        normal2_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "reflectionSampler" => {
                        if let Some(view) = hash_to_texture.get(&texture.hash) {
                            if view.texture().depth_or_array_layers() == 1 {
                                reflection_texture = Some(view);
                            }
                            reflection_sampler = Some(device.create_sampler(&sampler(texture)));
                        }
                    }
                    "reflectionCubeSampler" => {
                        if let Some(view) = hash_to_texture.get(&texture.hash) {
                            if view.texture().depth_or_array_layers() == 6 {
                                reflection_cube_texture = Some(view);
                            }
                        }
                        reflection_cube_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "color2Sampler" => {
                        color2_texture = hash_to_texture.get(&texture.hash);
                        color2_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "diffuseSampler" => {
                        diffuse_texture = hash_to_texture.get(&texture.hash);
                        diffuse_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "lightMapSampler" => {
                        light_map_texture = hash_to_texture.get(&texture.hash);
                        light_map_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    _ => (),
                }
            }
        } else {
            error!("Unable to find shader {:X} in database", material.shader_id);
        }
    }

    // TODO: Get sampler values from material textures.
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });

    let uniforms = device.create_uniform_buffer(
        "MC",
        &crate::shader::model::Uniforms {
            alpha_blend_params: get_parameter(mesh, "NU_alphaBlendParams").unwrap_or_default(),
            angle_fade_params: get_parameter(mesh, "NU_angleFadeParams").unwrap_or_default(),
            ao_min_gain: get_parameter(mesh, "NU_aoMinGain").unwrap_or_default(),
            color_gain: get_parameter(mesh, "NU_colorGain").unwrap_or_default(),
            color_offset: get_parameter(mesh, "NU_colorOffset").unwrap_or_default(),
            color_sampler2_u_v: get_parameter(mesh, "NU_colorSampler2UV").unwrap_or_default(),
            color_sampler3_u_v: get_parameter(mesh, "NU_colorSampler3UV").unwrap_or_default(),
            color_sampler4_u_v: get_parameter(mesh, "NU_colorSampler4UV").unwrap_or_default(),
            color_sampler_u_v: get_parameter(mesh, "NU_colorSamplerUV").unwrap_or_default(),
            color_step_u_v: get_parameter(mesh, "NU_colorStepUV").unwrap_or_default(),
            custom_soft_light_params: get_parameter(mesh, "NU_customSoftLightParams")
                .unwrap_or_default(),
            diffuse_color: get_parameter(mesh, "NU_diffuseColor").unwrap_or_default(),
            dual_normal_scroll_params: get_parameter(mesh, "NU_dualNormalScrollParams")
                .unwrap_or_default(),
            final_color_gain: get_parameter(mesh, "NU_finalColorGain").unwrap_or_default(),
            final_color_gain2: get_parameter(mesh, "NU_finalColorGain2").unwrap_or_default(),
            final_color_gain3: get_parameter(mesh, "NU_finalColorGain3").unwrap_or_default(),
            fog_params: get_parameter(mesh, "NU_fogParams").unwrap_or_default(),
            fresnel_color: get_parameter(mesh, "NU_fresnelColor").unwrap_or_default(),
            fresnel_params: get_parameter(mesh, "NU_fresnelParams").unwrap_or_default(),
            normal_params: get_parameter(mesh, "NU_normalParams").unwrap_or_default(),
            normal_sampler_a_u_v: get_parameter(mesh, "NU_normalSamplerAUV").unwrap_or_default(),
            normal_sampler_b_u_v: get_parameter(mesh, "NU_normalSamplerBUV").unwrap_or_default(),
            reflection_color: get_parameter(mesh, "NU_reflectionColor").unwrap_or_default(),
            reflection_params: get_parameter(mesh, "NU_reflectionParams").unwrap_or_default(),
            rotate_pivot_u_v: get_parameter(mesh, "NU_rotatePivotUV").unwrap_or_default(),
            soft_lighting_params: get_parameter(mesh, "NU_softLightingParams").unwrap_or_default(),
            specular_color: get_parameter(mesh, "NU_specularColor").unwrap_or_default(),
            specular_color_gain: get_parameter(mesh, "NU_specularColorGain").unwrap_or_default(),
            specular_params: get_parameter(mesh, "NU_specularParams").unwrap_or_default(),
            test_param0: get_parameter(mesh, "NU_testParam0").unwrap_or_default(),
            test_param1: get_parameter(mesh, "NU_testParam1").unwrap_or_default(),
            test_param2: get_parameter(mesh, "NU_testParam2").unwrap_or_default(),
            test_param3: get_parameter(mesh, "NU_testParam3").unwrap_or_default(),
            translucent_color: get_parameter(mesh, "NU_translucentColor").unwrap_or_default(),
            z_offset: get_parameter(mesh, "NU_zOffset").unwrap_or_default(),
        },
    );

    // TODO: Are these initialized differently than MC uniforms?
    let effect_uniforms = device.create_uniform_buffer(
        "MC_EFFECT",
        &crate::shader::model::EffectUniforms {
            angle_fade_params: get_parameter(mesh, "NU_angleFadeParams").unwrap_or_default(),
            eff_color_gain: get_parameter(mesh, "NU_effColorGain").unwrap_or_default(),
            eff_combiner_alpha0: get_parameter(mesh, "NU_effCombinerAlpha0").unwrap_or_default(),
            eff_combiner_color0: get_parameter(mesh, "NU_effCombinerColor0").unwrap_or_default(),
            eff_combiner_color1: get_parameter(mesh, "NU_effCombinerColor1").unwrap_or_default(),
            eff_depth_offset: get_parameter(mesh, "NU_effDepthOffset").unwrap_or_default(),
            eff_m_t_blend_alpha: get_parameter(mesh, "NU_effMTBlendAlpha").unwrap_or_default(),
            eff_m_t_blend_param0: get_parameter(mesh, "NU_effMTBlendParam0").unwrap_or_default(),
            eff_m_t_blend_param1: get_parameter(mesh, "NU_effMTBlendParam1").unwrap_or_default(),
            eff_m_t_blend_param2: get_parameter(mesh, "NU_effMTBlendParam2").unwrap_or_default(),
            eff_refract_param: get_parameter(mesh, "NU_effRefractParam").unwrap_or_default(),
            eff_rot_u_v: get_parameter(mesh, "NU_effRotUV").unwrap_or_default(),
            eff_scale_u_v: get_parameter(mesh, "NU_effScaleUV").unwrap_or_default(),
            eff_silhouette_color: get_parameter(mesh, "NU_effSilhouetteColor").unwrap_or_default(),
            eff_sun_shaft_params0: get_parameter(mesh, "NU_effSunShaftParams0").unwrap_or_default(),
            eff_sun_shaft_params1: get_parameter(mesh, "NU_effSunShaftParams1").unwrap_or_default(),
            eff_trans_u_v: get_parameter(mesh, "NU_effTransUV").unwrap_or_default(),
            eff_universe_param: get_parameter(mesh, "NU_effUniverseParam").unwrap_or_default(),
            eff_y_grad_color_bottom: get_parameter(mesh, "NU_effYGradColorBottom")
                .unwrap_or_default(),
            eff_y_grad_color_top: get_parameter(mesh, "NU_effYGradColorTop").unwrap_or_default(),
            eff_y_grad_param: get_parameter(mesh, "NU_effYGradParam").unwrap_or_default(),
            normal_params: get_parameter(mesh, "NU_normalParams").unwrap_or_default(),
            normal_sampler_a_u_v: get_parameter(mesh, "NU_normalSamplerAUV").unwrap_or_default(),
        },
    );

    crate::shader::model::bind_groups::BindGroup2::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout2 {
            uniforms: uniforms.as_entire_buffer_binding(),
            effect_uniforms: effect_uniforms.as_entire_buffer_binding(),
            color_texture: color_texture.unwrap_or(default_texture),
            color_sampler: color_sampler.as_ref().unwrap_or(&sampler),
            normal_texture: normal_texture.unwrap_or(default_texture),
            normal_sampler: normal_sampler.as_ref().unwrap_or(&sampler),
            reflection_texture: reflection_texture.unwrap_or(default_texture),
            reflection_sampler: reflection_sampler.as_ref().unwrap_or(&sampler),
            reflection_cube_texture: reflection_cube_texture.unwrap_or(default_cube_texture),
            reflection_cube_sampler: reflection_cube_sampler.as_ref().unwrap_or(&sampler),
            color2_texture: color2_texture.unwrap_or(default_texture),
            color2_sampler: color2_sampler.as_ref().unwrap_or(&sampler),
            diffuse_texture: diffuse_texture.unwrap_or(default_texture),
            diffuse_sampler: diffuse_sampler.as_ref().unwrap_or(&sampler),
            light_map_texture: light_map_texture.unwrap_or(default_texture),
            light_map_sampler: light_map_sampler.as_ref().unwrap_or(&sampler),
            normal2_texture: normal2_texture.unwrap_or(default_texture),
            normal2_sampler: normal2_sampler.as_ref().unwrap_or(&sampler),
        },
    )
}

fn sampler(texture: &sm4sh_model::NudTexture) -> wgpu::SamplerDescriptor<'_> {
    // TODO: set mipmaps and anisotropy
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: address_mode(texture.wrap_mode_s),
        address_mode_v: address_mode(texture.wrap_mode_t),
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: match texture.mag_filter {
            sm4sh_model::MagFilter::Unk0 => wgpu::FilterMode::Nearest,
            sm4sh_model::MagFilter::Nearest => wgpu::FilterMode::Nearest,
            sm4sh_model::MagFilter::Linear => wgpu::FilterMode::Linear,
        },
        min_filter: match texture.min_filter {
            sm4sh_model::MinFilter::LinearMipmapLinear => wgpu::FilterMode::Linear,
            sm4sh_model::MinFilter::Nearest => wgpu::FilterMode::Nearest,
            sm4sh_model::MinFilter::Linear => wgpu::FilterMode::Linear,
            sm4sh_model::MinFilter::NearestMipmapLinear => wgpu::FilterMode::Nearest,
        },
        ..Default::default()
    }
}

fn address_mode(m: sm4sh_model::WrapMode) -> wgpu::AddressMode {
    match m {
        sm4sh_model::WrapMode::Repeat => wgpu::AddressMode::Repeat,
        sm4sh_model::WrapMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        sm4sh_model::WrapMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
    }
}

fn get_parameter(mesh: &sm4sh_model::NudMesh, name: &str) -> Option<Vec4> {
    let material = mesh.material1.as_ref()?;
    material.properties.iter().find_map(|p| {
        if p.name == name {
            Some(vec4(
                *p.values.first()?,
                *p.values.get(1)?,
                *p.values.get(2)?,
                *p.values.get(3)?,
            ))
        } else {
            None
        }
    })
}
