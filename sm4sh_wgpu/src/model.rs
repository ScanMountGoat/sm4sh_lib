use glam::{vec3, Vec3};
use sm4sh_model::nud::{vertex::Normals, NudModel};
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
}

pub fn load_model(device: &wgpu::Device, nud: &NudModel) -> Model {
    Model {
        meshes: nud
            .groups
            .iter()
            .flat_map(|g| g.meshes.iter().map(|m| create_mesh(device, m)))
            .collect(),
    }
}

fn create_mesh(device: &wgpu::Device, m: &sm4sh_model::nud::NudMesh) -> Mesh {
    let mut vertices: Vec<_> = m
        .vertices
        .positions
        .iter()
        .map(|p| crate::shader::model::VertexInput0 {
            position: (*p).into(),
            normal: Vec3::ZERO,
        })
        .collect();

    set_normals(&m.vertices.normals, &mut vertices);

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

    Mesh {
        vertex_buffer,
        index_buffer,
        vertex_index_count: m.vertex_indices.len() as u32,
        is_strip: m.primitive_type == sm4sh_model::nud::PrimitiveType::TriangleStrip,
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
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.vertex_index_count, 0, 0..1);
        }
    }
}
