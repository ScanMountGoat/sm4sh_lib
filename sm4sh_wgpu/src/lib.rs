use encase::{ShaderSize, ShaderType, StorageBuffer, UniformBuffer, internal::WriteInto};
use glam::{Mat4, Vec4, vec2, vec4};
use sm4sh_model::database::ShaderDatabase;
use wgpu::util::DeviceExt;

mod material;
mod model;
mod pipeline;
mod renderer;
mod shader;
mod shadergen;
mod skeleton;
mod texture;

pub use model::{Mesh, Model, load_model};
pub use renderer::Renderer;

/// The features required by [Renderer].
pub const FEATURES: wgpu::Features = wgpu::Features::TEXTURE_COMPRESSION_BC
    .union(wgpu::Features::POLYGON_MODE_LINE)
    .union(wgpu::Features::TEXTURE_FORMAT_16BIT_NORM)
    .union(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);

trait DeviceBufferExt {
    fn create_uniform_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        contents: &T,
    ) -> wgpu::Buffer;
}

impl DeviceBufferExt for wgpu::Device {
    fn create_uniform_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        data: &T,
    ) -> wgpu::Buffer {
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&data).unwrap();

        // TODO: is it worth not adding COPY_DST to all buffers?
        self.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}

trait QueueBufferExt {
    fn write_uniform_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &T,
    );

    fn write_storage_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &[T],
    );
}

impl QueueBufferExt for wgpu::Queue {
    fn write_uniform_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &T,
    ) {
        let mut bytes = UniformBuffer::new(Vec::new());
        bytes.write(&data).unwrap();

        self.write_buffer(buffer, 0, &bytes.into_inner());
    }

    fn write_storage_data<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        buffer: &wgpu::Buffer,
        data: &[T],
    ) {
        let mut bytes = StorageBuffer::new(Vec::new());
        bytes.write(&data).unwrap();

        self.write_buffer(buffer, 0, &bytes.into_inner());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraData {
    pub view: Mat4,
    pub projection: Mat4,
    pub view_projection: Mat4,
    pub position: Vec4,
    pub width: u32,
    pub height: u32,
}

impl CameraData {
    fn to_shader_data(self) -> shader::model::Camera {
        // Undo the view rotation without affecting translation.
        let mut view_rot_inv_billboard = self.view.inverse();
        *view_rot_inv_billboard.col_mut(3) = vec4(0.0, 0.0, 0.0, 1.0);

        // Always point up for the y-axis.
        let mut view_rot_inv_billboard_y = view_rot_inv_billboard;
        *view_rot_inv_billboard_y.col_mut(1) = vec4(0.0, 1.0, 0.0, 0.0);

        crate::shader::model::Camera {
            view: self.view,
            projection: self.projection,
            view_projection: self.view_projection,
            view_rot_inv_billboard,
            view_rot_inv_billboard_y,
            position: self.position,
            resolution: vec2(self.width as f32, self.height as f32),
        }
    }
}

pub struct SharedData {
    model_layout: wgpu::PipelineLayout,
    database: ShaderDatabase,
}

impl SharedData {
    pub fn new(device: &wgpu::Device, database: ShaderDatabase) -> Self {
        // TODO: Include database in binary?
        Self {
            model_layout: crate::shader::model::create_pipeline_layout(device),
            database,
        }
    }
}
