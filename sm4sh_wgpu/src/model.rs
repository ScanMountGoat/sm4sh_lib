use std::collections::BTreeMap;

use glam::{vec2, vec3, Vec2, Vec3};
use sm4sh_model::nud::{
    vertex::{Normals, Uvs},
    NudModel,
};
use wgpu::util::DeviceExt;

pub struct Model {
    meshes: Vec<Mesh>,
}

// TODO: Is it worth grouping meshes?
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
    is_strip: bool,

    bind_group1: crate::shader::model::bind_groups::BindGroup1,
}

pub fn load_model(device: &wgpu::Device, queue: &wgpu::Queue, model: &NudModel) -> Model {
    // TODO: texture module
    let textures = model
        .textures
        .iter()
        .map(|t| {
            (
                t.hash_id,
                device
                    .create_texture_with_data(
                        queue,
                        &wgpu::TextureDescriptor {
                            label: Some(&format!("{:x}", t.hash_id)),
                            size: wgpu::Extent3d {
                                width: t.width,
                                height: t.height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: t.mipmap_count,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: texture_format(t.image_format),
                            usage: wgpu::TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                        wgpu::util::TextureDataOrder::LayerMajor,
                        &t.image_data,
                    )
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        })
        .collect();

    Model {
        meshes: model
            .groups
            .iter()
            .flat_map(|g| g.meshes.iter().map(|m| create_mesh(device, m, &textures)))
            .collect(),
    }
}

fn create_mesh(
    device: &wgpu::Device,
    m: &sm4sh_model::nud::NudMesh,
    hash_to_texture: &BTreeMap<u32, wgpu::TextureView>,
) -> Mesh {
    let mut vertices: Vec<_> = m
        .vertices
        .positions
        .iter()
        .map(|p| crate::shader::model::VertexInput0 {
            position: (*p).into(),
            normal: Vec3::ZERO,
            uv0: Vec2::ZERO,
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
    // TODO: Avoid unwrap and use default textures.
    let texture_hash = m
        .material1
        .as_ref()
        .and_then(|m| m.texture_hashes.first())
        .unwrap();

    let color_texture = hash_to_texture.get(texture_hash).unwrap();

    // TODO: Get sampler values from material textures.
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });

    let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            color_texture,
            color_sampler: &sampler,
        },
    );

    Mesh {
        vertex_buffer,
        index_buffer,
        vertex_index_count: m.vertex_indices.len() as u32,
        is_strip: m.primitive_type == sm4sh_model::nud::PrimitiveType::TriangleStrip,
        bind_group1,
    }
}

fn set_normals(
    normals: &sm4sh_model::nud::vertex::Normals,
    vertices: &mut [crate::shader::model::VertexInput0],
) {
    match normals {
        Normals::None(_) => (),
        Normals::NormalsFloat32(items) => set_attribute(vertices, items, |v, i| {
            v.normal = vec3(i.normal[0], i.normal[1], i.normal[2])
        }),
        Normals::Unk2(items) => set_attribute(vertices, items, |v, i| {
            v.normal = vec3(i.normal[0], i.normal[1], i.normal[2])
        }),
        Normals::NormalsTangentBitangentFloat32(items) => set_attribute(vertices, items, |v, i| {
            v.normal = vec3(i.normal[0], i.normal[1], i.normal[2])
        }),
        Normals::NormalsFloat16(items) => set_attribute(vertices, items, |v, i| {
            v.normal = vec3(
                i.normal[0].to_f32(),
                i.normal[1].to_f32(),
                i.normal[2].to_f32(),
            )
        }),
        Normals::NormalsTangentBitangentFloat16(items) => set_attribute(vertices, items, |v, i| {
            v.normal = vec3(
                i.normal[0].to_f32(),
                i.normal[1].to_f32(),
                i.normal[2].to_f32(),
            )
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
                v.uv0 = vec2(i.u.to_f32(), i.v.to_f32())
            }),
            Uvs::Float32(items) => set_attribute(vertices, items, |v, i| v.uv0 = vec2(i.u, i.v)),
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

fn texture_format(image_format: sm4sh_model::nud::NutFormat) -> wgpu::TextureFormat {
    match image_format {
        sm4sh_model::nud::NutFormat::Bc1 => wgpu::TextureFormat::Bc1RgbaUnorm,
        sm4sh_model::nud::NutFormat::Bc2 => wgpu::TextureFormat::Bc2RgbaUnorm,
        sm4sh_model::nud::NutFormat::Bc3 => wgpu::TextureFormat::Bc3RgbaUnorm,
        sm4sh_model::nud::NutFormat::Unk6 => todo!(),
        sm4sh_model::nud::NutFormat::Rg16 => wgpu::TextureFormat::Bc1RgbaUnorm,
        sm4sh_model::nud::NutFormat::Rgba16 => wgpu::TextureFormat::Rgba16Unorm,
        sm4sh_model::nud::NutFormat::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
        sm4sh_model::nud::NutFormat::Bgra8 => wgpu::TextureFormat::Bgra8Unorm,
        sm4sh_model::nud::NutFormat::Rgba82 => todo!(),
        sm4sh_model::nud::NutFormat::Unk22 => todo!(),
    }
}

impl Model {
    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass,
        list_pipeline: &wgpu::RenderPipeline,
        strip_pipeline: &wgpu::RenderPipeline,
    ) {
        for mesh in &self.meshes {
            if mesh.is_strip {
                render_pass.set_pipeline(strip_pipeline);
            } else {
                render_pass.set_pipeline(list_pipeline);
            }

            mesh.bind_group1.set(render_pass);

            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.vertex_index_count, 0, 0..1);
        }
    }
}
