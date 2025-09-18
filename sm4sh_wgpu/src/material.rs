use std::collections::BTreeMap;

use log::error;

use glam::{vec4, Vec4};
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
            for ((_, s), texture) in program.samplers.iter().zip(&material.textures) {
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
                        reflection_texture = hash_to_texture.get(&texture.hash);
                        reflection_sampler = Some(device.create_sampler(&sampler(texture)));
                    }
                    "reflectionCubeSampler" => {
                        reflection_cube_texture = hash_to_texture.get(&texture.hash);
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

    // TODO: Use snake case for these uniforms?
    let uniforms = device.create_uniform_buffer(
        "MC",
        &crate::shader::model::Uniforms {
            alphaBlendParams: get_parameter(mesh, "NU_alphaBlendParams").unwrap_or_default(),
            angleFadeParams: get_parameter(mesh, "NU_angleFadeParams").unwrap_or_default(),
            aoMinGain: get_parameter(mesh, "NU_aoMinGain").unwrap_or_default(),
            colorGain: get_parameter(mesh, "NU_colorGain").unwrap_or_default(),
            colorOffset: get_parameter(mesh, "NU_colorOffset").unwrap_or_default(),
            colorSampler2UV: get_parameter(mesh, "NU_colorSampler2UV").unwrap_or_default(),
            colorSampler3UV: get_parameter(mesh, "NU_colorSampler3UV").unwrap_or_default(),
            colorSamplerUV: get_parameter(mesh, "NU_colorSamplerUV").unwrap_or_default(),
            colorStepUV: get_parameter(mesh, "NU_colorStepUV").unwrap_or_default(),
            customSoftLightParams: get_parameter(mesh, "NU_customSoftLightParams")
                .unwrap_or_default(),
            diffuseColor: get_parameter(mesh, "NU_diffuseColor").unwrap_or_default(),
            dualNormalScrollParams: get_parameter(mesh, "NU_dualNormalScrollParams")
                .unwrap_or_default(),
            finalColorGain: get_parameter(mesh, "NU_finalColorGain").unwrap_or_default(),
            finalColorGain2: get_parameter(mesh, "NU_finalColorGain2").unwrap_or_default(),
            finalColorGain3: get_parameter(mesh, "NU_finalColorGain3").unwrap_or_default(),
            fogParams: get_parameter(mesh, "NU_fogParams").unwrap_or_default(),
            fresnelColor: get_parameter(mesh, "NU_fresnelColor").unwrap_or_default(),
            fresnelParams: get_parameter(mesh, "NU_fresnelParams").unwrap_or_default(),
            normalParams: get_parameter(mesh, "NU_normalParams").unwrap_or_default(),
            normalSamplerAUV: get_parameter(mesh, "NU_normalSamplerAUV").unwrap_or_default(),
            normalSamplerBUV: get_parameter(mesh, "NU_normalSamplerBUV").unwrap_or_default(),
            reflectionColor: get_parameter(mesh, "NU_reflectionColor").unwrap_or_default(),
            reflectionParams: get_parameter(mesh, "NU_reflectionParams").unwrap_or_default(),
            rotatePivotUV: get_parameter(mesh, "NU_rotatePivotUV").unwrap_or_default(),
            softLightingParams: get_parameter(mesh, "NU_softLightingParams").unwrap_or_default(),
            specularColor: get_parameter(mesh, "NU_specularColor").unwrap_or_default(),
            specularColorGain: get_parameter(mesh, "NU_specularColorGain").unwrap_or_default(),
            specularParams: get_parameter(mesh, "NU_specularParams").unwrap_or_default(),
            zOffset: get_parameter(mesh, "NU_zOffset").unwrap_or_default(),
        },
    );

    let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout2 {
            uniforms: uniforms.as_entire_buffer_binding(),
            color_texture: color_texture.unwrap_or(default_texture),
            color_sampler: color_sampler.as_ref().unwrap_or(&sampler),
            normal_texture: normal_texture.unwrap_or(default_texture),
            normal_sampler: normal_sampler.as_ref().unwrap_or(&sampler),
            reflection_texture: reflection_texture.unwrap_or(default_texture),
            reflection_sampler: reflection_sampler.as_ref().unwrap_or(&sampler),
            // TODO: Correctly initialize cube textures.
            reflection_cube_texture: default_cube_texture,
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
    );
    bind_group2
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
