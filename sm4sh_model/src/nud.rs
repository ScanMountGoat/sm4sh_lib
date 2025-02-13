use std::io::{Cursor, Seek, Write};

use binrw::BinResult;
use sm4sh_lib::{
    nud::{BoundingSphere, Nud},
    nut::Nut,
};
use vertex::{read_vertex_indices, read_vertices, write_vertex_indices, write_vertices, Vertices};

pub mod vertex;

#[derive(Debug, PartialEq, Clone)]
pub struct NudModel {
    pub groups: Vec<NudMeshGroup>,
    pub textures: Vec<ImageTexture>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMeshGroup {
    pub meshes: Vec<NudMesh>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMesh {
    // Assume meshes have unique vertex data.
    pub vertices: Vertices,
    pub vertex_indices: Vec<u16>,
    pub primitive_type: PrimitiveType,
    // TODO: material?
}

#[derive(Debug, PartialEq, Clone)]
pub enum PrimitiveType {
    TriangleList,
    TriangleStrip,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImageTexture {}

impl NudModel {
    pub fn from_nud(nud: &Nud, nut: &Nut) -> BinResult<Self> {
        let mut groups = Vec::new();

        let mut mesh_index = 0;
        for g in &nud.mesh_groups {
            let mut meshes = Vec::new();
            for _ in 0..g.mesh_count {
                let mesh = &nud.meshes[mesh_index];

                // TODO: Avoid potential indexing panics.
                let vertices = read_vertices(
                    &nud.vertex_buffer0[mesh.vertex_buffer0_offset as usize..],
                    &nud.vertex_buffer1[mesh.vertex_buffer1_offset as usize..],
                    mesh.vertex_flags,
                    mesh.uv_color_flags,
                    mesh.vertex_count,
                )?;

                let vertex_indices = read_vertex_indices(
                    &nud.index_buffer[mesh.vertex_indices_offset as usize..],
                    mesh.vertex_index_count,
                )?;

                let primitive_type = if mesh.vertex_index_flags.is_triangle_list() {
                    PrimitiveType::TriangleList
                } else {
                    PrimitiveType::TriangleStrip
                };

                meshes.push(NudMesh {
                    vertices,
                    vertex_indices,
                    primitive_type,
                });

                mesh_index += 1;
            }

            groups.push(NudMeshGroup { meshes });
        }

        // TODO: deswizzle nut textures.
        let textures = Vec::new();

        Ok(Self { groups, textures })
    }

    pub fn to_nud(&self) -> BinResult<Nud> {
        let mut buffer0 = Cursor::new(Vec::new());
        let mut buffer1 = Cursor::new(Vec::new());
        let mut index_buffer = Cursor::new(Vec::new());

        let mesh_groups = Vec::new();
        let meshes = Vec::new();

        for group in &self.groups {
            for mesh in &group.meshes {
                let (vertex_flags, uv_color_flags) =
                    write_vertices(&mesh.vertices, &mut buffer0, &mut buffer1)?;

                write_vertex_indices(&mut index_buffer, &mesh.vertex_indices)?;
            }
        }

        align(&mut buffer0, 16, 0u8)?;
        align(&mut buffer1, 16, 0u8)?;
        align(&mut index_buffer, 16, 0u8)?;

        let vertex_buffer0 = buffer0.into_inner();
        let vertex_buffer1 = buffer1.into_inner();
        let index_buffer = index_buffer.into_inner();

        // TODO: Fill in remaining fields.
        Ok(Nud {
            file_size: 0,
            version: 0,
            mesh_group_count: self.groups.len() as u16,
            bone_start_index: 0,
            bone_end_index: 0,
            indices_offset: 0,
            indices_size: index_buffer.len() as u32,
            vertex_buffer0_size: vertex_buffer0.len() as u32,
            vertex_buffer1_size: vertex_buffer1.len() as u32,
            bounding_sphere: BoundingSphere {
                center: [0.0; 3],
                radius: 0.0,
            },
            mesh_groups,
            meshes,
            index_buffer,
            vertex_buffer0,
            vertex_buffer1,
        })
    }
}

fn align<W: Write + Seek>(writer: &mut W, align: u64, pad: u8) -> Result<(), std::io::Error> {
    let size = writer.stream_position()?;
    let aligned_size = size.next_multiple_of(align);
    let padding = aligned_size - size;
    writer.write_all(&vec![pad; padding as usize])?;
    Ok(())
}
