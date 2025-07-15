use std::io::{Cursor, Seek, Write};

use binrw::BinResult;
use glam::{Vec3, Vec4, Vec4Swizzles};
use sm4sh_lib::{
    nud::{
        BoundingSphere, Material, MaterialFlags, MaterialTexture, Mesh, MeshGroup, Nud,
        VertexIndexFlags,
    },
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
    // TODO: Better way to store skeletons?
    pub bone_start_index: usize,
    pub bone_end_index: usize,
    // TODO: Create a type for this.
    pub bounding_sphere: Vec4,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NudMeshGroup {
    pub name: String,
    pub meshes: Vec<NudMesh>,
    pub sort_bias: f32,
    pub bounding_sphere: Vec4,
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

            groups.push(NudMeshGroup {
                name: g.name.clone(),
                meshes,
                sort_bias: g.sort_bias,
                bounding_sphere: Vec3::from(g.bounding_sphere.center)
                    .extend(g.bounding_sphere.radius),
            });
        }

        let textures = nut_textures(nut);

        Ok(Self {
            groups,
            textures,
            bone_start_index: nud.bone_start_index as usize,
            bone_end_index: nud.bone_end_index as usize,
            bounding_sphere: Vec3::from(nud.bounding_sphere.center)
                .extend(nud.bounding_sphere.radius),
        })
    }

    pub fn to_nud(&self) -> BinResult<Nud> {
        let mut mesh_groups = Vec::new();
        let mut meshes = Vec::new();

        let mut buffer0 = Cursor::new(Vec::new());
        let mut buffer1 = Cursor::new(Vec::new());
        let mut index_buffer = Cursor::new(Vec::new());

        for group in &self.groups {
            for mesh in &group.meshes {
                let vertex_buffer0_offset = buffer0.position() as u32;
                let vertex_buffer1_offset = buffer1.position() as u32;
                let vertex_indices_offset = index_buffer.position() as u32;

                let (vertex_flags, uv_color_flags) =
                    write_vertices(&mesh.vertices, &mut buffer0, &mut buffer1)?;
                align(&mut buffer0, 16, 0u8)?;
                align(&mut buffer1, 16, 0u8)?;

                write_vertex_indices(&mut index_buffer, &mesh.vertex_indices)?;

                meshes.push(Mesh {
                    vertex_indices_offset,
                    vertex_buffer0_offset,
                    vertex_buffer1_offset,
                    vertex_count: mesh.vertices.positions.len() as u16,
                    vertex_flags,
                    uv_color_flags,
                    material1: mesh.material1.as_ref().map(material),
                    material2: mesh.material2.as_ref().map(material),
                    material3: mesh.material3.as_ref().map(material),
                    material4: mesh.material4.as_ref().map(material),
                    vertex_index_count: mesh.vertex_indices.len() as u16,
                    vertex_index_flags: VertexIndexFlags::new(
                        false,
                        false,
                        false,
                        0u8.into(),
                        true,
                        false,
                    ),
                    unk: [0; 3],
                });
            }

            mesh_groups.push(MeshGroup {
                bounding_sphere: bounding_sphere(group.bounding_sphere),
                center: group.bounding_sphere.xyz().to_array(),
                sort_bias: group.sort_bias,
                name: group.name.clone(),
                unk1: 0,
                bone_flag: 0,
                parent_bone_index: -1,
                mesh_count: group.meshes.len() as u16,
                position: 0,
            });
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
            version: 512,
            mesh_group_count: self.groups.len() as u16,
            bone_start_index: self.bone_start_index as u16,
            bone_end_index: self.bone_end_index as u16,
            indices_offset: 0,
            indices_size: index_buffer.len() as u32,
            vertex_buffer0_size: vertex_buffer0.len() as u32,
            vertex_buffer1_size: vertex_buffer1.len() as u32,
            bounding_sphere: bounding_sphere(self.bounding_sphere),
            mesh_groups,
            meshes,
            index_buffer,
            vertex_buffer0,
            vertex_buffer1,
        })
    }
}

fn material(m: &NudMaterial) -> Material {
    Material {
        flags: m.flags,
        unk1: 0,
        src_factor: m.src_factor,
        tex_count: m.texture_hashes.len() as u16,
        dst_factor: m.dst_factor,
        alpha_func: m.alpha_func,
        ref_alpha: 0,
        cull_mode: m.cull_mode,
        unk2: 0,
        unk3: 0,
        z_buffer_offset: 0,
        textures: m
            .texture_hashes
            .iter()
            .map(|hash| MaterialTexture {
                hash: *hash,
                unk1: [0; 3],
                map_mode: sm4sh_lib::nud::MapMode::TexCoord,
                wrap_mode_s: sm4sh_lib::nud::WrapMode::ClampToEdge,
                wrap_mode_t: sm4sh_lib::nud::WrapMode::ClampToEdge,
                min_filter: sm4sh_lib::nud::MinFilter::Linear,
                mag_filter: sm4sh_lib::nud::MagFilter::Linear,
                mip_detail: sm4sh_lib::nud::MipDetail::FourMipLevels,
                unk2: 0,
                unk3: 0,
                unk4: 0,
            })
            .collect(),
        properties: Vec::new(),
    }
}

fn bounding_sphere(sphere: Vec4) -> BoundingSphere {
    BoundingSphere {
        center: sphere.xyz().to_array(),
        radius: sphere.w,
    }
}

fn nut_textures(nut: &Nut) -> Vec<ImageTexture> {
    match nut {
        Nut::Ntwu(ntwu) => ntwu
            .textures
            .iter()
            .map(|t| ImageTexture::from_surface(t.gidx.hash, t.to_surface().unwrap()))
            .collect(),
        Nut::Ntp3(ntp3) => ntp3
            .textures
            .iter()
            .map(|t| ImageTexture::from_surface(t.gidx.hash, t.to_surface().unwrap()))
            .collect(),
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

impl ImageTexture {
    /// Create a view of all image data in this texture
    /// to use with encode or decode operations.
    pub fn to_surface(&self) -> image_dds::Surface<&[u8]> {
        image_dds::Surface {
            width: self.width,
            height: self.height,
            depth: 1,
            layers: 1,
            mipmaps: self.mipmap_count,
            image_format: self.image_format.into(),
            data: &self.image_data,
        }
    }

    pub fn from_surface<T: AsRef<[u8]>>(hash_id: u32, surface: image_dds::Surface<T>) -> Self {
        Self {
            hash_id,
            width: surface.width,
            height: surface.height,
            mipmap_count: surface.mipmaps,
            image_format: surface.image_format.into(),
            image_data: surface.data.as_ref().to_vec(),
        }
    }
}
