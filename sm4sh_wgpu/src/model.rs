use sm4sh_model::nud::NudModel;
use wgpu::util::DeviceExt;

pub struct Model {
    meshes: Vec<Mesh>,
}

// TODO: Is it worth grouping meshes?
pub struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_index_count: u32,
}

pub fn load_model(device: &wgpu::Device, nud: &NudModel) -> Model {
    Model {
        meshes: nud
            .groups
            .iter()
            .flat_map(|g| {
                g.meshes.iter().map(|m| {
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("vertex buffer"),
                            contents: bytemuck::cast_slice(&m.vertices.positions),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                    let index_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("index buffer"),
                            contents: bytemuck::cast_slice(&m.vertex_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });

                    Mesh {
                        vertex_buffer,
                        index_buffer,
                        vertex_index_count: m.vertex_indices.len() as u32,
                    }
                })
            })
            .collect(),
    }
}

impl Model {
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass) {
        for mesh in &self.meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.vertex_index_count, 0, 0..1);
        }
    }
}
