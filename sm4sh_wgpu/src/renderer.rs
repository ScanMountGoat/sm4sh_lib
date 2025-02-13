use glam::{Mat4, Vec4};
use sm4sh_model::nud::PrimitiveType;

use crate::{CameraData, DeviceBufferExt, Model, QueueBufferExt};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    camera_buffer: wgpu::Buffer,
    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,
    model_pipeline_triangle_list: wgpu::RenderPipeline,
    model_pipeline_triangle_strip: wgpu::RenderPipeline,
    textures: Textures,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        // TODO: Should models store the pipelines instead?
        let model_pipeline_triangle_list =
            model_pipeline(device, output_format, wgpu::PrimitiveTopology::TriangleList);
        let model_pipeline_triangle_strip = model_pipeline(
            device,
            output_format,
            wgpu::PrimitiveTopology::TriangleStrip,
        );

        let camera = CameraData {
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
            view_projection: Mat4::IDENTITY,
            position: Vec4::ZERO,
            width,
            height,
        };
        let camera_buffer = device.create_uniform_buffer("camera buffer", &camera.to_shader_data());

        let model_bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
            },
        );

        let textures = Textures::new(device, width, height);

        Self {
            camera_buffer,
            model_pipeline_triangle_list,
            model_pipeline_triangle_strip,
            model_bind_group0,
            textures,
        }
    }

    pub fn render_model(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        model: &Model,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        crate::shader::model::set_bind_groups(&mut render_pass, &self.model_bind_group0);
        model.draw(
            &mut render_pass,
            &self.model_pipeline_triangle_list,
            &self.model_pipeline_triangle_strip,
        );
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera_data: &CameraData) {
        queue.write_uniform_data(&self.camera_buffer, &camera_data.to_shader_data());
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        // Update each resource that depends on window size.
        self.textures = Textures::new(device, width, height);
    }
}

// Group resizable resources to avoid duplicating this logic.
pub struct Textures {
    depth: wgpu::TextureView,
}

impl Textures {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let depth = create_texture(device, width, height, "depth_texture", DEPTH_FORMAT);

        Self { depth }
    }
}

fn create_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    texture.create_view(&Default::default())
}

fn model_pipeline(
    device: &wgpu::Device,
    output_format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    let module = crate::shader::model::create_shader_module(device);
    let pipeline_layout = crate::shader::model::create_pipeline_layout(device);
    let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: crate::shader::model::vertex_state(
            &module,
            &crate::shader::model::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: topology.is_strip().then_some(wgpu::IndexFormat::Uint16),
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
            &crate::shader::model::fs_main_entry([Some(output_format.into())]),
        )),
        multiview: None,
        cache: None,
    });
    model_pipeline
}
