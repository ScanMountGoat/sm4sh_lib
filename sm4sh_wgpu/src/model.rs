use std::collections::{BTreeMap, HashMap};

use glam::{Mat4, UVec4, Vec4, Vec4Swizzles};
use sm4sh_model::{
    DstFactor, NudModel, SrcFactor, VbnSkeleton,
    vertex::{Bones, Colors, Normals, Uvs},
};
use wgpu::util::DeviceExt;

use crate::{
    CameraData, DeviceBufferExt, QueueBufferExt, SharedData,
    material::create_bind_group2,
    pipeline::{ShaderKey, model_pipeline},
    texture::create_texture,
};

pub struct Model {
    groups: Vec<MeshGroup>,

    skeleton: Option<VbnSkeleton>,
    pub(crate) bone_transforms: wgpu::Buffer,
    pub(crate) skinning_transforms: wgpu::Buffer,
    pub(crate) skinning_transforms_inv_transpose: wgpu::Buffer,
    pub(crate) bone_count: u32,

    bind_group1: crate::shader::model::bind_groups::BindGroup1,
}

pub struct MeshGroup {
    sort_bias: f32,
    bounding_sphere: Vec4,
    meshes: Vec<Mesh>,
}

// TODO: Is it worth grouping meshes?
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,

    is_transparent: bool,

    pipeline: wgpu::RenderPipeline,

    bind_group2: crate::shader::model::bind_groups::BindGroup2,
    bind_group3: crate::shader::model::bind_groups::BindGroup3,
}

pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &NudModel,
    shared_data: &SharedData,
) -> Model {
    let default_texture = create_solid_texture(device, queue, [0u8; 4])
        .create_view(&wgpu::TextureViewDescriptor::default());

    let default_cube_texture = create_default_black_cube_texture(device, queue).create_view(
        &wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        },
    );

    // TODO: texture module
    let mut textures: BTreeMap<_, _> = model
        .textures
        .iter()
        .map(|t| {
            (
                t.hash_id,
                create_texture(device, queue, t)
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            )
        })
        .collect();

    // TODO: proper loading for global textures.
    let light_map = create_solid_texture(device, queue, [255u8; 4])
        .create_view(&wgpu::TextureViewDescriptor::default());
    textures.insert(0x10080000, light_map);

    let bone_transforms = model
        .skeleton
        .as_ref()
        .map(|s| s.model_space_transforms())
        .unwrap_or(vec![Mat4::IDENTITY]);
    let skinning_transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("skinning transforms buffer"),
        contents: bytemuck::cast_slice(&vec![Mat4::IDENTITY; bone_transforms.len()]),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let skinning_transforms_inv_transpose =
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("skinning transforms inverse transpose buffer"),
            contents: bytemuck::cast_slice(&vec![Mat4::IDENTITY; bone_transforms.len()]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
    let bone_transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("bone transforms buffer"),
        contents: bytemuck::cast_slice(&bone_transforms),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let bone_count = model
        .skeleton
        .as_ref()
        .map(|s| s.bones.len() as u32)
        .unwrap_or_default();

    let bind_group1 = crate::shader::model::bind_groups::BindGroup1::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout1 {
            skinning_transforms: skinning_transforms.as_entire_buffer_binding(),
            skinning_transforms_inv_transpose: skinning_transforms_inv_transpose
                .as_entire_buffer_binding(),
        },
    );

    let mut shader_cache = HashMap::new();

    Model {
        groups: model
            .groups
            .iter()
            .map(|g| MeshGroup {
                meshes: g
                    .meshes
                    .iter()
                    .map(|m| {
                        create_mesh(
                            device,
                            g,
                            m,
                            &textures,
                            &default_texture,
                            &default_cube_texture,
                            shared_data,
                            &mut shader_cache,
                        )
                    })
                    .collect(),
                sort_bias: g.sort_bias,
                bounding_sphere: g.bounding_sphere,
            })
            .collect(),
        bone_transforms,
        skinning_transforms,
        skinning_transforms_inv_transpose,
        bone_count,
        skeleton: model.skeleton.clone(),
        bind_group1,
    }
}

fn create_mesh(
    device: &wgpu::Device,
    group: &sm4sh_model::NudMeshGroup,
    mesh: &sm4sh_model::NudMesh,
    hash_to_texture: &BTreeMap<u32, wgpu::TextureView>,
    default_texture: &wgpu::TextureView,
    default_cube_texture: &wgpu::TextureView,
    shared_data: &SharedData,
    shader_cache: &mut HashMap<Option<ShaderKey>, wgpu::ShaderModule>,
) -> Mesh {
    let mut vertices: Vec<_> = mesh
        .vertices
        .positions
        .iter()
        .map(|p| crate::shader::model::VertexInput0 {
            position: p.extend(1.0),
            normal: Vec4::ZERO,
            tangent: Vec4::ZERO,
            bitangent: Vec4::ZERO,
            color: Vec4::splat(0.5),
            indices: UVec4::ZERO,
            weights: Vec4::ZERO,
            uv01: Vec4::ZERO,
            uv23: Vec4::ZERO,
        })
        .collect();

    if let Some(bones) = &mesh.vertices.bones {
        set_bones(bones, &mut vertices);
    }
    set_normals(&mesh.vertices.normals, &mut vertices);
    set_uvs(&mesh.vertices.uvs, &mut vertices);
    if let Some(colors) = &mesh.vertices.colors {
        set_colors(colors, &mut vertices);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vertex buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index buffer"),
        contents: bytemuck::cast_slice(&mesh.vertex_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let bind_group2 = create_bind_group2(
        device,
        mesh,
        hash_to_texture,
        default_texture,
        default_cube_texture,
        shared_data,
    );

    let per_mesh = device.create_uniform_buffer(
        "PerMesh",
        &crate::shader::model::PerMesh {
            parent_bone: group.parent_bone_index.map(|i| i as i32).unwrap_or(-1),
            has_skinning: group.meshes.iter().any(|m| m.vertices.bones.is_some()) as u32,
        },
    );

    let bind_group3 = crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_mesh: per_mesh.as_entire_buffer_binding(),
        },
    );

    let pipeline = model_pipeline(device, shared_data, mesh, shader_cache);

    let is_transparent = mesh
        .material1
        .as_ref()
        .map(|m| m.src_factor != SrcFactor::One || m.dst_factor != DstFactor::Zero)
        .unwrap_or_default();

    Mesh {
        vertex_buffer,
        index_buffer,
        vertex_index_count: mesh.vertex_indices.len() as u32,
        bind_group2,
        bind_group3,
        pipeline,
        is_transparent,
    }
}

fn set_bones(bones: &Bones, vertices: &mut [crate::shader::model::VertexInput0]) {
    set_attribute(vertices, &bones.bone_indices, |v, i| {
        v.indices = (*i).into();
    });
    set_attribute(vertices, &bones.weights, |v, i| {
        v.weights = *i;
    });
}

fn set_normals(normals: &Normals, vertices: &mut [crate::shader::model::VertexInput0]) {
    match normals {
        Normals::None(_) => (),
        Normals::NormalsFloat32(items) => set_attribute(vertices, items, |v, i| {
            v.normal = i.normal.into();
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

fn set_uvs(uvs: &Uvs, vertices: &mut [crate::shader::model::VertexInput0]) {
    match uvs {
        Uvs::Float16(items) => {
            let mut uvs = items.iter();
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv01.x = i.u.to_f32();
                    v.uv01.y = i.v.to_f32();
                });
            }
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv01.z = i.u.to_f32();
                    v.uv01.w = i.v.to_f32();
                });
            }
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv23.x = i.u.to_f32();
                    v.uv23.y = i.v.to_f32();
                });
            }
        }
        Uvs::Float32(items) => {
            let mut uvs = items.iter();
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv01.x = i.u;
                    v.uv01.y = i.v;
                });
            }
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv01.z = i.u;
                    v.uv01.w = i.v;
                });
            }
            if let Some(uv) = uvs.next() {
                set_attribute(vertices, uv, |v, i| {
                    v.uv23.x = i.u;
                    v.uv23.y = i.v;
                });
            }
        }
    }
}

fn set_colors(colors: &Colors, vertices: &mut [crate::shader::model::VertexInput0]) {
    set_attribute(vertices, &colors.colors, |v, i| {
        v.color = *i;
    });
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
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass, camera: &CameraData) {
        // TODO: opaque sorted front to back?
        // TODO: transparent sorted back to front?
        let mut sorted: Vec<_> = self.groups.iter().collect();
        sorted.sort_by_key(|g| {
            // Render farther objects first.
            let camera_distance = camera.position.xyz().distance(g.bounding_sphere.xyz());
            let distance = -camera_distance + g.sort_bias;
            ordered_float::OrderedFloat::from(distance)
        });

        let (transparent, opaque): (Vec<_>, Vec<_>) = sorted
            .into_iter()
            .flat_map(|g| &g.meshes)
            .partition(|m| m.is_transparent);

        self.bind_group1.set(render_pass);

        for mesh in opaque {
            mesh.draw(render_pass);
        }
        // Transparent meshes are rendered after opaque meshes for proper blending.
        for mesh in transparent {
            mesh.draw(render_pass);
        }
    }

    pub fn update_bone_transforms(
        &self,
        queue: &wgpu::Queue,
        animation: &sm4sh_model::animation::Animation,
        frame: f32,
    ) {
        if let Some(skeleton) = &self.skeleton {
            // TODO: make looping optional?
            let final_frame = animation.frame_count.saturating_sub(1) as f32;
            let frame = frame.rem_euclid(final_frame);

            let skinning_transforms = animation.skinning_transforms(skeleton, frame);
            queue.write_storage_data(&self.skinning_transforms, &skinning_transforms);

            let skinning_transforms_inv_transpose: Vec<_> = skinning_transforms
                .iter()
                .map(|t| t.inverse().transpose())
                .collect();
            queue.write_storage_data(
                &self.skinning_transforms_inv_transpose,
                &skinning_transforms_inv_transpose,
            );

            let transforms = animation.model_space_transforms(skeleton, frame);
            queue.write_storage_data(&self.bone_transforms, &transforms);
        }
    }
}

impl Mesh {
    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        self.bind_group2.set(render_pass);
        self.bind_group3.set(render_pass);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.vertex_index_count, 0, 0..1);
    }
}

pub fn create_solid_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: [u8; 4],
) -> wgpu::Texture {
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
        bytemuck::cast_slice(&[rgba; 4 * 4]),
    )
}

pub fn create_default_black_cube_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::Texture {
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("DEFAULT_CUBE"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &[0u8; 4 * 4 * 4 * 6],
    )
}
