use std::collections::BTreeMap;

use glam::{vec4, Vec4};
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
    let default_texture = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

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
            .flat_map(|g| {
                g.meshes
                    .iter()
                    .map(|m| create_mesh(device, m, &textures, &default_texture))
            })
            .collect(),
    }
}

fn create_mesh(
    device: &wgpu::Device,
    m: &sm4sh_model::nud::NudMesh,
    hash_to_texture: &BTreeMap<u32, wgpu::TextureView>,
    default_texture: &wgpu::TextureView,
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
        let mut texture_index = 0;
        if material.flags.unk1().diffuse() {
            color_texture = hash_to_texture.get(&material.texture_hashes[texture_index]);
            texture_index += 1;
        }
        if material.flags.unk1().sphere() {
            texture_index += 1;
        }
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

    let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            color_texture: color_texture.unwrap_or(default_texture),
            normal_texture: normal_texture.unwrap_or(default_texture),
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
