use glam::{Mat4, Vec4};

use crate::{skeleton::BoneRenderer, CameraData, DeviceBufferExt, Model, QueueBufferExt};

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    camera_buffer: wgpu::Buffer,
    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,
    textures: Textures,
    bone_renderer: BoneRenderer,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
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

        let bone_renderer = BoneRenderer::new(device, &camera_buffer, output_format);

        Self {
            camera_buffer,
            model_bind_group0,
            textures,
            bone_renderer,
        }
    }

    pub fn render_model(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        model: &Model,
        camera: &CameraData,
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

        self.model_bind_group0.set(&mut render_pass);
        model.draw(&mut render_pass, camera);

        self.bone_renderer
            .draw_bones(&mut render_pass, &model.bone_transforms, model.bone_count);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &CameraData) {
        queue.write_uniform_data(&self.camera_buffer, &camera.to_shader_data());
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
