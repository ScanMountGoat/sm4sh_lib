use std::collections::BTreeMap;

use glam::{ivec4, vec4, Mat4, UVec4, Vec4, Vec4Swizzles};
use log::error;
use sm4sh_model::{
    vertex::{Bones, Colors, Normals, Uvs},
    DstFactor, NudMesh, NudModel, SrcFactor, VbnSkeleton,
};
use wgpu::util::DeviceExt;

use crate::{
    renderer::DEPTH_FORMAT, texture::create_texture, CameraData, DeviceBufferExt, QueueBufferExt,
    SharedData,
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

    pipeline: wgpu::RenderPipeline,

    bind_group2: crate::shader::model::bind_groups::BindGroup2,
    bind_group3: crate::shader::model::bind_groups::BindGroup3,
}

pub fn load_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &NudModel,
    output_format: wgpu::TextureFormat,
    shared_data: &SharedData,
) -> Model {
    let default_texture = create_default_black_texture(device, queue)
        .create_view(&wgpu::TextureViewDescriptor::default());

    // TODO: texture module
    let textures = model
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
                            output_format,
                            shared_data,
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
    output_format: wgpu::TextureFormat,
    shared_data: &SharedData,
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
            uv0: Vec4::ZERO,
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

    // TODO: Load all textures and samplers.
    let mut color_texture = None;
    let mut normal_texture = None;

    if let Some(material) = &mesh.material1 {
        // TODO: Just use integers for keys in the database?
        if let Some(program) = shared_data
            .database
            .programs
            .get(&format!("{:X}", material.shader_id))
        {
            for ((_, s), texture) in program.samplers.iter().zip(&material.textures) {
                match s.as_str() {
                    "colorSampler" => color_texture = hash_to_texture.get(&texture.hash),
                    "normalSampler" => normal_texture = hash_to_texture.get(&texture.hash),
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
        "Uniforms",
        &crate::shader::model::Uniforms {
            has_normal_map: normal_texture.is_some() as u32,
        },
    );

    let bind_group2 = crate::shader::model::bind_groups::BindGroup2::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout2 {
            color_texture: color_texture.unwrap_or(default_texture),
            normal_texture: normal_texture.unwrap_or(default_texture),
            color_sampler: &sampler,
            uniforms: uniforms.as_entire_buffer_binding(),
        },
    );

    let per_mesh = device.create_uniform_buffer(
        "PerMesh",
        &crate::shader::model::PerMesh {
            parent_bone: ivec4(
                group.parent_bone_index.map(|i| i as i32).unwrap_or(-1),
                0,
                0,
                0,
            ),
        },
    );

    let bind_group3 = crate::shader::model::bind_groups::BindGroup3::from_bindings(
        device,
        crate::shader::model::bind_groups::BindGroupLayout3 {
            per_mesh: per_mesh.as_entire_buffer_binding(),
        },
    );

    let pipeline = model_pipeline(device, output_format, shared_data, mesh);

    Mesh {
        vertex_buffer,
        index_buffer,
        vertex_index_count: mesh.vertex_indices.len() as u32,
        bind_group2,
        bind_group3,
        pipeline,
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
        Uvs::Float16(items) => set_attribute(vertices, &items[0], |v, i| {
            v.uv0 = vec4(i.u.to_f32(), i.v.to_f32(), 0.0, 0.0);
        }),
        Uvs::Float32(items) => {
            set_attribute(vertices, &items[0], |v, i| v.uv0 = vec4(i.u, i.v, 0.0, 0.0));
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

        self.bind_group1.set(render_pass);

        for group in &sorted {
            for mesh in &group.meshes {
                render_pass.set_pipeline(&mesh.pipeline);
                mesh.bind_group2.set(render_pass);
                mesh.bind_group3.set(render_pass);

                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.vertex_index_count, 0, 0..1);
            }
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
    shared_data: &SharedData,
    mesh: &NudMesh,
) -> wgpu::RenderPipeline {
    let topology = match mesh.primitive_type {
        sm4sh_model::PrimitiveType::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        sm4sh_model::PrimitiveType::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    };

    // TODO: Add unit tests for returning state structs.
    let blend = mesh.material1.as_ref().map(blend_state);

    let strip_index_format = topology.is_strip().then_some(wgpu::IndexFormat::Uint16);

    let cull_mode = mesh.material1.as_ref().and_then(|m| match m.cull_mode {
        sm4sh_model::CullMode::Disabled => None,
        sm4sh_model::CullMode::Outside => Some(wgpu::Face::Front),
        sm4sh_model::CullMode::Inside => Some(wgpu::Face::Back),
        sm4sh_model::CullMode::Disabled2 => None,
        sm4sh_model::CullMode::Inside2 => Some(wgpu::Face::Front),
        sm4sh_model::CullMode::Outside2 => Some(wgpu::Face::Back),
    });

    // TODO: alpha testing.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Pipeline"),
        layout: Some(&shared_data.model_layout),
        vertex: crate::shader::model::vertex_state(
            &shared_data.model_shader,
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
            &shared_data.model_shader,
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

fn blend_state(m: &sm4sh_model::NudMaterial) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: match m.src_factor {
                SrcFactor::One => wgpu::BlendFactor::One,
                SrcFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::One2 => wgpu::BlendFactor::One,
                SrcFactor::SourceAlpha2 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::Zero => wgpu::BlendFactor::Zero,
                SrcFactor::SourceAlpha3 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
                SrcFactor::DestinationAlpha7 => wgpu::BlendFactor::DstAlpha,
                SrcFactor::DestinationColor => wgpu::BlendFactor::Dst,
                SrcFactor::SrcAlpha3 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::SrcAlpha4 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::Unk16 => wgpu::BlendFactor::One,
                SrcFactor::Unk33 => wgpu::BlendFactor::One,
                SrcFactor::SrcAlpha5 => wgpu::BlendFactor::SrcAlpha,
            },
            dst_factor: match m.dst_factor {
                DstFactor::Zero => wgpu::BlendFactor::Zero,
                DstFactor::OneMinusSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One => wgpu::BlendFactor::One,
                DstFactor::OneReverseSubtract => wgpu::BlendFactor::One,
                DstFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
                DstFactor::SourceAlphaReverseSubtract => wgpu::BlendFactor::SrcAlpha,
                DstFactor::OneMinusDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
                DstFactor::One2 => wgpu::BlendFactor::One,
                DstFactor::Zero2 => wgpu::BlendFactor::Zero,
                DstFactor::Unk10 => wgpu::BlendFactor::Zero,
                DstFactor::OneMinusSourceAlpha2 => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One3 => wgpu::BlendFactor::One,
                DstFactor::Zero5 => wgpu::BlendFactor::Zero,
                DstFactor::Zero3 => wgpu::BlendFactor::Zero,
                DstFactor::One4 => wgpu::BlendFactor::One,
                DstFactor::OneMinusSourceAlpha3 => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One5 => wgpu::BlendFactor::One,
            },
            operation: match m.dst_factor {
                DstFactor::Zero => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha => wgpu::BlendOperation::Add,
                DstFactor::One => wgpu::BlendOperation::Add,
                DstFactor::OneReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
                DstFactor::SourceAlpha => wgpu::BlendOperation::Add,
                DstFactor::SourceAlphaReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
                DstFactor::OneMinusDestinationAlpha => wgpu::BlendOperation::Add,
                DstFactor::One2 => wgpu::BlendOperation::Add,
                DstFactor::Zero2 => wgpu::BlendOperation::Add,
                DstFactor::Unk10 => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha2 => wgpu::BlendOperation::Add,
                DstFactor::One3 => wgpu::BlendOperation::Add,
                DstFactor::Zero5 => wgpu::BlendOperation::Add,
                DstFactor::Zero3 => wgpu::BlendOperation::Add,
                DstFactor::One4 => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha3 => wgpu::BlendOperation::Add,
                DstFactor::One5 => wgpu::BlendOperation::Add,
            },
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}
