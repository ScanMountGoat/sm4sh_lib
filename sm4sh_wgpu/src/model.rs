use std::collections::BTreeMap;

use glam::{vec4, Vec4};
use sm4sh_model::nud::{
    vertex::{Normals, Uvs},
    NudMesh, NudModel,
};
use wgpu::util::DeviceExt;

use crate::{renderer::DEPTH_FORMAT, texture::create_texture, DeviceBufferExt};

pub struct Model {
    meshes: Vec<Mesh>,
}

// TODO: Is it worth grouping meshes?
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,

    pipeline: wgpu::RenderPipeline,

    bind_group1: crate::shader::model::bind_groups::BindGroup1,
}

pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &NudModel,
    output_format: wgpu::TextureFormat,
) -> Model {
    let default_texture = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    // TODO: texture module
    let textures = model
        .textures
        .iter()
        .map(|t| {
            // TODO: Why do final mipmaps not work for some non square textures?
            let mut data = t.image_data.clone();
            data.resize(data.len() + 32, 0u8);
            (
                t.hash_id,
                create_texture(device, queue, t)
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        })
        .collect();

    Model {
        meshes: model
            .groups
            .iter()
            .flat_map(|g| {
                g.meshes
                    .iter()
                    .map(|m| create_mesh(device, m, &textures, &default_texture, output_format))
            })
            .collect(),
    }
}

fn create_mesh(
    device: &wgpu::Device,
    m: &sm4sh_model::nud::NudMesh,
    hash_to_texture: &BTreeMap<u32, wgpu::TextureView>,
    default_texture: &wgpu::TextureView,
    output_format: wgpu::TextureFormat,
) -> Mesh {
    let mut vertices: Vec<_> = m
        .vertices
        .positions
        .iter()
        .map(|p| crate::shader::model::VertexInput0 {
            position: vec4(p[0], p[1], p[2], 1.0),
            normal: Vec4::ZERO,
            tangent: Vec4::ZERO,
            bitangent: Vec4::ZERO,
            uv0: Vec4::ZERO,
        })
        .collect();

    set_normals(&m.vertices.normals, &mut vertices);
    set_uvs(&m.vertices.uvs, &mut vertices);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vertex buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index buffer"),
        contents: bytemuck::cast_slice(&m.vertex_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // TODO: Load all textures and samplers.
    let mut color_texture = None;
    let mut normal_texture = None;

    if let Some(material) = &m.material1 {
        // TODO: Is this the "correct" way to detect texture types?
        // TODO: Cross reference this with shader uniforms?
        // TODO: Unit tests for all known 4th bytes?
        let mut texture_index = 0;
        if material.flags.unk1().diffuse() {
            color_texture = hash_to_texture.get(&material.texture_hashes[texture_index]);
            texture_index += 1;
        }
        if material.flags.unk1().sphere() {
            texture_index += 1;
        }
        // TODO: has stage cube?
        // TODO: has cube = has ramp or cube and not dummy ramp and not sphere map
        if material.flags.unk1().normal() {
            normal_texture = hash_to_texture.get(&material.texture_hashes[texture_index]);
            texture_index += 1;
        }
        if material.flags.unk1().ramp_or_cube() {
            texture_index += 1;
        }
        if material.flags.unk1().dummy_ramp() {
            texture_index += 1;
        }
    }

    // TODO: Get sampler values from material textures.
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });

    let uniforms = device.create_uniform_buffer(
        "Uniforms",
        &crate::shader::model::Uniforms {
            has_normal_map: normal_texture.is_some() as u32,
        },
    );

    let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            color_texture: color_texture.unwrap_or(default_texture),
            normal_texture: normal_texture.unwrap_or(default_texture),
            color_sampler: &sampler,
            uniforms: uniforms.as_entire_buffer_binding(),
        },
    );

    let pipeline = model_pipeline(device, output_format, m);

    Mesh {
        vertex_buffer,
        index_buffer,
        vertex_index_count: m.vertex_indices.len() as u32,
        bind_group1,
        pipeline,
    }
}

fn set_normals(
    normals: &sm4sh_model::nud::vertex::Normals,
    vertices: &mut [crate::shader::model::VertexInput0],
) {
    match normals {
        Normals::None(_) => (),
        Normals::NormalsFloat32(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.into();
        }),
        Normals::Unk2(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.into();
            v.tangent = i.tangent.into();
            v.bitangent = i.bitangent.into();
        }),
        Normals::NormalsTangentBitangentFloat32(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.into();
            v.tangent = i.tangent.into();
            v.bitangent = i.bitangent.into();
        }),
        Normals::NormalsFloat16(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.map(|f| f.to_f32()).into();
        }),
        Normals::NormalsTangentBitangentFloat16(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.map(|f| f.to_f32()).into();
            v.tangent = i.tangent.map(|f| f.to_f32()).into();
            v.bitangent = i.bitangent.map(|f| f.to_f32()).into();
        }),
    }
}

fn set_uvs(
    uvs: &[sm4sh_model::nud::vertex::Uvs],
    vertices: &mut [crate::shader::model::VertexInput0],
) {
    if let Some(uvs) = uvs.first() {
        match uvs {
            Uvs::Float16(items) => set_attribute(vertices, items, |v, i| {
                v.uv0 = vec4(i.u.to_f32(), i.v.to_f32(), 0.0, 0.0)
            }),
            Uvs::Float32(items) => {
                set_attribute(vertices, items, |v, i| v.uv0 = vec4(i.u, i.v, 0.0, 0.0))
            }
        }
    }
}

fn set_attribute<T, F>(
    vertices: &mut [crate::shader::model::VertexInput0],
    items: &[T],
    set_attribute: F,
) where
    F: Fn(&mut crate::shader::model::VertexInput0, &T),
{
    for (v, i) in vertices.iter_mut().zip(items) {
        set_attribute(v, i);
    }
}

impl Model {
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        for mesh in &self.meshes {
            render_pass.set_pipeline(&mesh.pipeline);
            mesh.bind_group1.set(render_pass);

            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.vertex_index_count, 0, 0..1);
        }
    }
}

pub fn create_default_black_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("DEFAULT"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &[0u8; 4 * 4 * 4],
    )
}

fn model_pipeline(
    device: &wgpu::Device,
    output_format: wgpu::TextureFormat,
    mesh: &NudMesh,
) -> wgpu::RenderPipeline {
    let module = crate::shader::model::create_shader_module(device);
    let pipeline_layout = crate::shader::model::create_pipeline_layout(device);

    let topology = match mesh.primitive_type {
        sm4sh_model::nud::PrimitiveType::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        sm4sh_model::nud::PrimitiveType::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    };

    // TODO: Add unit tests for returning state structs.
    let blend = mesh.material1.as_ref().map(blend_state);

    let strip_index_format = topology.is_strip().then_some(wgpu::IndexFormat::Uint16);

    let cull_mode = mesh.material1.as_ref().and_then(|m| match m.cull_mode {
        sm4sh_model::nud::CullMode::Disabled => None,
        sm4sh_model::nud::CullMode::Outside => Some(wgpu::Face::Front),
        sm4sh_model::nud::CullMode::Inside => Some(wgpu::Face::Back),
        sm4sh_model::nud::CullMode::Disabled2 => None,
        sm4sh_model::nud::CullMode::Inside2 => Some(wgpu::Face::Front),
        sm4sh_model::nud::CullMode::Outside2 => Some(wgpu::Face::Back),
    });

    // TODO: alpha testing.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: crate::shader::model::vertex_state(
            &module,
            &crate::shader::model::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format,
            cull_mode,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(crate::shader::model::fragment_state(
            &module,
            &crate::shader::model::fs_main_entry([Some(wgpu::ColorTargetState {
                format: output_format,
                blend,
                write_mask: wgpu::ColorWrites::all(),
            })]),
        )),
        multiview: None,
        cache: None,
    })
}

fn blend_state(m: &sm4sh_model::nud::NudMaterial) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: match m.src_factor {
                sm4sh_model::nud::SrcFactor::One => wgpu::BlendFactor::One,
                sm4sh_model::nud::SrcFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
                sm4sh_model::nud::SrcFactor::One2 => wgpu::BlendFactor::One,
                sm4sh_model::nud::SrcFactor::SourceAlpha2 => wgpu::BlendFactor::SrcAlpha,
                sm4sh_model::nud::SrcFactor::Zero => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::SrcFactor::SourceAlpha3 => wgpu::BlendFactor::SrcAlpha,
                sm4sh_model::nud::SrcFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
                sm4sh_model::nud::SrcFactor::DestinationAlpha7 => wgpu::BlendFactor::DstAlpha,
                sm4sh_model::nud::SrcFactor::DestinationColor => wgpu::BlendFactor::Dst,
                sm4sh_model::nud::SrcFactor::Unk11 => todo!(),
                sm4sh_model::nud::SrcFactor::Unk15 => todo!(),
                sm4sh_model::nud::SrcFactor::Unk16 => todo!(),
                sm4sh_model::nud::SrcFactor::Unk33 => todo!(),
                sm4sh_model::nud::SrcFactor::Unk37 => todo!(),
            },
            dst_factor: match m.dst_factor {
                sm4sh_model::nud::DstFactor::Zero => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::OneMinusSourceAlpha => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::One => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::OneReverseSubtract => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::SourceAlpha => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::SourceAlphaReverseSubtract => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::OneMinusDestinationAlpha => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::One2 => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::Zero2 => wgpu::BlendFactor::Zero,
                sm4sh_model::nud::DstFactor::Unk10 => todo!(),
                sm4sh_model::nud::DstFactor::Unk11 => todo!(),
                sm4sh_model::nud::DstFactor::Unk12 => todo!(),
                sm4sh_model::nud::DstFactor::Unk64 => todo!(),
                sm4sh_model::nud::DstFactor::Unk112 => todo!(),
                sm4sh_model::nud::DstFactor::Unk114 => todo!(),
                sm4sh_model::nud::DstFactor::Unk129 => todo!(),
                sm4sh_model::nud::DstFactor::Unk130 => todo!(),
            },
            operation: match m.dst_factor {
                sm4sh_model::nud::DstFactor::Zero => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::OneMinusSourceAlpha => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::One => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::OneReverseSubtract => {
                    wgpu::BlendOperation::ReverseSubtract
                }
                sm4sh_model::nud::DstFactor::SourceAlpha => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::SourceAlphaReverseSubtract => {
                    wgpu::BlendOperation::ReverseSubtract
                }
                sm4sh_model::nud::DstFactor::OneMinusDestinationAlpha => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::One2 => wgpu::BlendOperation::Add,
                sm4sh_model::nud::DstFactor::Zero2 => todo!(),
                sm4sh_model::nud::DstFactor::Unk10 => todo!(),
                sm4sh_model::nud::DstFactor::Unk11 => todo!(),
                sm4sh_model::nud::DstFactor::Unk12 => todo!(),
                sm4sh_model::nud::DstFactor::Unk64 => todo!(),
                sm4sh_model::nud::DstFactor::Unk112 => todo!(),
                sm4sh_model::nud::DstFactor::Unk114 => todo!(),
                sm4sh_model::nud::DstFactor::Unk129 => todo!(),
                sm4sh_model::nud::DstFactor::Unk130 => todo!(),
            },
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}
