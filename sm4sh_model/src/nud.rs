use std::io::{Cursor, Seek, Write};

use binrw::BinResult;
use sm4sh_lib::{
    nud::{BoundingSphere, MaterialFlags, Nud},
    nut::Nut,
};
use vertex::{read_vertex_indices, read_vertices, write_vertex_indices, write_vertices, Vertices};

pub use sm4sh_lib::nud::{AlphaFunc, CullMode, DstFactor, SrcFactor};
pub use sm4sh_lib::nut::NutFormat;

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
    pub material1: Option<NudMaterial>,
    pub material2: Option<NudMaterial>,
    pub material3: Option<NudMaterial>,
    pub material4: Option<NudMaterial>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMaterial {
    // TODO: Should this recreate flags or store them directly?
    pub flags: MaterialFlags,

    pub src_factor: SrcFactor,
    pub dst_factor: DstFactor,
    pub alpha_func: AlphaFunc,
    pub cull_mode: CullMode,

    pub texture_hashes: Vec<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PrimitiveType {
    TriangleList,
    TriangleStrip,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImageTexture {
    pub hash_id: u32,
    pub width: u32,
    pub height: u32,
    pub mipmap_count: u32,
    pub image_format: NutFormat,
    pub image_data: Vec<u8>,
}

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
                    material1: mesh.material1.as_ref().map(nud_material),
                    material2: mesh.material2.as_ref().map(nud_material),
                    material3: mesh.material3.as_ref().map(nud_material),
                    material4: mesh.material4.as_ref().map(nud_material),
                });

                mesh_index += 1;
            }

            groups.push(NudMeshGroup { meshes });
        }

        let textures = match nut {
            Nut::Ntwu(ntwu) => ntwu
                .textures
                .iter()
                .map(|t| ImageTexture {
                    hash_id: t.gidx.hash,
                    width: t.width as u32,
                    height: t.height as u32,
                    mipmap_count: t.mipmap_count as u32,
                    image_format: t.format,
                    image_data: t.deswizzle().unwrap(),
                })
                .collect(),
            Nut::Ntp3(ntp3) => ntp3
                .textures
                .iter()
                .map(|t| ImageTexture {
                    hash_id: t.gidx.hash,
                    width: t.width as u32,
                    height: t.height as u32,
                    mipmap_count: t.mipmap_count as u32,
                    image_format: t.format,
                    image_data: t.deswizzle().unwrap(),
                })
                .collect(),
        };

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

fn nud_material(material: &sm4sh_lib::nud::Material) -> NudMaterial {
    NudMaterial {
        flags: material.flags,
        src_factor: material.src_factor,
        dst_factor: material.dst_factor,
        alpha_func: material.alpha_func,
        cull_mode: material.cull_mode,
        texture_hashes: material.textures.iter().map(|t| t.hash).collect(),
    }
}
