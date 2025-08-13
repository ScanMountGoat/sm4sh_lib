use encase::{internal::WriteInto, ShaderSize, ShaderType, StorageBuffer, UniformBuffer};
use glam::{vec2, Mat4, Vec4};
use sm4sh_model::shader_database::ShaderDatabase;
use wgpu::util::DeviceExt;

mod model;
mod renderer;
mod shader;
mod skeleton;
mod texture;

pub use model::{load_model, Mesh, Model};
pub use renderer::Renderer;

/// The features required by [Renderer].
pub const FEATURES: wgpu::Features =
    wgpu::Features::TEXTURE_COMPRESSION_BC.union(wgpu::Features::POLYGON_MODE_LINE);

trait DeviceBufferExt {
    fn create_uniform_buffer<T: ShaderType + WriteInto + ShaderSize>(
        &self,
        label: &str,
        contents: &T,
    ) -> wgpu::Buffer;

    fn create_storage_buffer<T>(&self, label: &str, contents: &[T]) -> wgpu::Buffer
    where
        [T]: WriteInto + ShaderType;
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

    fn create_storage_buffer<T>(&self, label: &str, data: &[T]) -> wgpu::Buffer
    where
        [T]: WriteInto + ShaderType,
    {
        let mut buffer = StorageBuffer::new(Vec::new());
        buffer.write(data).unwrap();

        // TODO: is it worth not adding COPY_DST to all buffers?
        self.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
        crate::shader::model::Camera {
            view: self.view,
            projection: self.projection,
            view_projection: self.view_projection,
            position: self.position,
            resolution: vec2(self.width as f32, self.height as f32),
        }
    }
}

pub struct SharedData {
    model_layout: wgpu::PipelineLayout,
    model_shader: wgpu::ShaderModule,
    database: ShaderDatabase,
}

impl SharedData {
    pub fn new(device: &wgpu::Device, database: ShaderDatabase) -> Self {
        // TODO: Include database in binary?
        Self {
            model_layout: crate::shader::model::create_pipeline_layout(device),
            model_shader: crate::shader::model::create_shader_module(device),
            database,
        }
    }
}
