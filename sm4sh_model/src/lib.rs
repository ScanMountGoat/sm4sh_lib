use std::{
    borrow::Cow,
    collections::BTreeSet,
    io::{Cursor, Seek, Write},
    path::Path,
};
use vertex::{
    Vertices, buffer0_stride, buffer1_stride, read_vertex_indices, read_vertices,
    triangle_strip_to_list, write_vertex_indices, write_vertices,
};

use binrw::BinResult;
use glam::{EulerRot, Mat4, Vec3, Vec4, Vec4Swizzles};
use sm4sh_lib::{
    nud::{
        BoundingSphere, Material, MaterialProperty, MaterialTexture, Mesh, MeshGroup, Nud,
        VertexIndexFlags,
    },
    nut::{CreateSurfaceError, Nut},
    vbn::Vbn,
};

pub use sm4sh_lib::nud::{
    AlphaFunc, BoneFlags, CullMode, DstFactor, MagFilter, MapMode, MinFilter, MipDetail, SrcFactor,
    WrapMode,
};
pub use sm4sh_lib::nut::NutFormat;
pub use sm4sh_lib::vbn::BoneType;

pub mod animation;
pub mod database;
pub mod skinning;
pub mod vertex;

/// Load a nud model from `path` and the corresponding `"model.nut"` and `"model.vbn"` if present.
#[tracing::instrument(skip_all)]
pub fn load_model<P: AsRef<Path>>(path: P) -> BinResult<NudModel> {
    let path = path.as_ref();
    let nud = Nud::from_file(path)?;

    // TODO: Better error reporting.
    let nut_path = path.with_file_name("model.nut");
    let nut = Nut::from_file(&nut_path).ok().or_else(|| {
        // Some older nut files use zlib compression.
        // This isn't used by any in game nut files but is supported by Smash Forge.
        let bytes = std::fs::read(nut_path).ok()?;
        let decompressed = zune_inflate::DeflateDecoder::new(&bytes)
            .decode_zlib()
            .ok()?;
        Nut::from_bytes(&decompressed).ok()
    });

    let vbn = Vbn::from_file(path.with_file_name("model.vbn")).ok();
    NudModel::from_nud(&nud, nut.as_ref(), vbn.as_ref())
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudModel {
    pub groups: Vec<NudMeshGroup>,
    pub textures: Vec<ImageTexture>,
    pub bounding_sphere: Vec4, // TODO: Create a type for bounding spheres.
    pub skeleton: Option<VbnSkeleton>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudMeshGroup {
    pub name: String,
    pub meshes: Vec<NudMesh>,
    pub sort_bias: f32,
    pub bounding_sphere: Vec4,
    pub parent_bone_index: Option<usize>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudMesh {
    // Assume meshes have unique vertex data.
    pub vertices: Vertices,
    pub vertex_indices: Vec<u16>,
    pub primitive_type: PrimitiveType,
    pub material1: Option<NudMaterial>,
    pub material2: Option<NudMaterial>,
    pub material3: Option<NudMaterial>,
    pub material4: Option<NudMaterial>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudMaterial {
    pub shader_id: u32,
    pub src_factor: SrcFactor,
    pub dst_factor: DstFactor,
    pub alpha_func: AlphaFunc,
    pub alpha_test_ref: u16,
    pub cull_mode: CullMode,
    pub textures: Vec<NudTexture>,
    pub properties: Vec<NudProperty>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudTexture {
    pub hash: u32,
    pub map_mode: MapMode,
    pub wrap_mode_s: WrapMode,
    pub wrap_mode_t: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub mip_detail: MipDetail,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct NudProperty {
    pub name: String,
    pub values: Vec<f32>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum PrimitiveType {
    TriangleList,
    TriangleStrip,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct ImageTexture {
    pub hash_id: u32,
    pub width: u32,
    pub height: u32,
    pub mipmap_count: u32,
    pub layers: u32,
    pub image_format: NutFormat,
    pub image_data: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct VbnSkeleton {
    pub bones: Vec<VbnBone>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct VbnBone {
    pub name: String,
    pub hash: u32,
    pub parent_bone_index: Option<usize>,
    pub bone_type: BoneType,
    pub translation: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl NudModel {
    pub fn from_nud(nud: &Nud, nut: Option<&Nut>, vbn: Option<&Vbn>) -> BinResult<Self> {
        let mut groups = Vec::new();

        for g in &nud.mesh_groups {
            let mut meshes = Vec::new();
            for mesh in &g.meshes {
                let vertices = read_vertices(
                    &nud.vertex_buffer0,
                    mesh.vertex_buffer0_offset,
                    &nud.vertex_buffer1,
                    mesh.vertex_buffer1_offset,
                    mesh.vertex_flags,
                    mesh.vertex_count,
                )?;

                let vertex_indices = read_vertex_indices(
                    &nud.index_buffer,
                    mesh.vertex_indices_offset,
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
            }

            groups.push(NudMeshGroup {
                name: g.name.clone(),
                meshes,
                sort_bias: g.sort_bias,
                bounding_sphere: Vec3::from(g.bounding_sphere.center)
                    .extend(g.bounding_sphere.radius),
                parent_bone_index: usize::try_from(g.parent_bone_index).ok(),
            });
        }

        // TODO: Return errors.
        let textures = nut.and_then(|n| nut_textures(n).ok()).unwrap_or_default();

        let skeleton = vbn.map(vbn_skeleton);

        Ok(Self {
            groups,
            textures,
            bounding_sphere: Vec3::from(nud.bounding_sphere.center)
                .extend(nud.bounding_sphere.radius),
            skeleton,
        })
    }

    pub fn to_nud(&self) -> BinResult<Nud> {
        let mut mesh_groups = Vec::new();

        let mut buffer0 = Cursor::new(Vec::new());
        let mut buffer1 = Cursor::new(Vec::new());
        let mut index_buffer = Cursor::new(Vec::new());

        let mut used_bone_indices = BTreeSet::new();

        for group in &self.groups {
            if let Some(index) = group.parent_bone_index {
                used_bone_indices.insert(index as u32);
            }

            let mut meshes = Vec::new();
            for mesh in &group.meshes {
                let vertex_buffer0_offset = buffer0.position() as u32;
                let vertex_buffer1_offset = buffer1.position() as u32;
                let vertex_indices_offset = index_buffer.position() as u32;

                let vertex_flags = write_vertices(&mesh.vertices, &mut buffer0, &mut buffer1)?;
                // TODO: Why is this not always aligned?
                align(&mut buffer0, 16, 0u8)?;
                align(&mut buffer1, 16, 0u8)?;

                write_vertex_indices(&mut index_buffer, &mesh.vertex_indices)?;

                // TODO: Is there a nicer way of setting offsets to 0?
                let stride0 = buffer0_stride(vertex_flags);
                let stride1 = buffer1_stride(vertex_flags);

                let vertex_buffer0_offset = if stride0 == 0 {
                    0
                } else {
                    vertex_buffer0_offset
                };
                let vertex_buffer1_offset = if stride1 == 0 {
                    0
                } else {
                    vertex_buffer1_offset
                };

                if let Some(indices) = mesh.vertices.bones.as_ref().map(|b| &b.bone_indices) {
                    used_bone_indices.extend(indices.iter().flatten());
                }

                meshes.push(Mesh {
                    vertex_indices_offset,
                    vertex_buffer0_offset,
                    vertex_buffer1_offset,
                    vertex_count: mesh.vertices.positions.len() as u16,
                    vertex_flags,
                    material1: mesh.material1.as_ref().map(material),
                    material2: mesh.material2.as_ref().map(material),
                    material3: mesh.material3.as_ref().map(material),
                    material4: mesh.material4.as_ref().map(material),
                    vertex_index_count: mesh.vertex_indices.len() as u16,
                    vertex_index_flags: VertexIndexFlags::new(
                        false,
                        false,
                        vertex_flags.bones() != sm4sh_lib::nud::BoneType::None,
                        0u8.into(),
                        mesh.primitive_type == PrimitiveType::TriangleList,
                        false,
                    ),
                    unk: [0; 3],
                });
            }

            let bone_flags = if group.parent_bone_index.is_some() {
                BoneFlags::ParentBone
            } else if group.meshes.iter().any(|m| m.vertices.bones.is_some()) {
                BoneFlags::Skinning
            } else {
                BoneFlags::Disabled
            };

            mesh_groups.push(MeshGroup {
                bounding_sphere: bounding_sphere(group.bounding_sphere),
                center: group.bounding_sphere.xyz().to_array(),
                sort_bias: group.sort_bias,
                name: group.name.clone(),
                unk1: 0,
                bone_flags,
                parent_bone_index: group.parent_bone_index.map(|i| i as i16).unwrap_or(-1),
                mesh_count: meshes.len() as u16,
                meshes,
            });
        }

        align(&mut buffer0, 16, 0u8)?;
        align(&mut buffer1, 16, 0u8)?;
        align(&mut index_buffer, 16, 0u8)?;

        let vertex_buffer0 = buffer0.into_inner();
        let vertex_buffer1 = buffer1.into_inner();
        let index_buffer = index_buffer.into_inner();

        let bone_start_index = used_bone_indices.iter().copied().min().unwrap_or_default() as u16;
        let bone_end_index = used_bone_indices.iter().copied().max().unwrap_or_default() as u16;

        Ok(Nud {
            file_size: 0,
            version: 512,
            mesh_group_count: self.groups.len() as u16,
            bone_start_index,
            bone_end_index,
            indices_offset: 0,
            indices_size: index_buffer.len() as u32,
            vertex_buffer0_size: vertex_buffer0.len() as u32,
            vertex_buffer1_size: vertex_buffer1.len() as u32,
            bounding_sphere: bounding_sphere(self.bounding_sphere),
            mesh_groups,
            index_buffer,
            vertex_buffer0,
            vertex_buffer1,
        })
    }
}

impl NudMesh {
    pub fn triangle_list_indices(&self) -> Cow<'_, [u16]> {
        match self.primitive_type {
            PrimitiveType::TriangleList => Cow::Borrowed(&self.vertex_indices),
            PrimitiveType::TriangleStrip => {
                Cow::Owned(triangle_strip_to_list(&self.vertex_indices))
            }
        }
    }
}

impl VbnBone {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_euler(
                EulerRot::XYZEx,
                self.rotation.x,
                self.rotation.y,
                self.rotation.z,
            )
            * Mat4::from_scale(self.scale)
    }
}

impl VbnSkeleton {
    /// The global transform for each bone in model space
    /// by recursively applying the parent transform.
    ///
    /// This is also known as the bone's "rest pose" or "bind pose".
    /// For inverse bind matrices, invert each matrix.
    pub fn model_space_transforms(&self) -> Vec<Mat4> {
        let mut final_transforms: Vec<_> = self.bones.iter().map(|b| b.matrix()).collect();

        // TODO: Don't assume bones appear after their parents.
        for i in 0..final_transforms.len() {
            if let Some(parent) = self.bones[i].parent_bone_index {
                final_transforms[i] = final_transforms[parent] * self.bones[i].matrix();
            }
        }

        final_transforms
    }
}

fn vbn_skeleton(vbn: &Vbn) -> VbnSkeleton {
    let (bones, transforms) = match vbn {
        Vbn::Le(vbn) => (&vbn.bones, &vbn.transforms),
        Vbn::Be(vbn) => (&vbn.bones, &vbn.transforms),
    };
    VbnSkeleton {
        bones: bones
            .iter()
            .zip(transforms)
            .map(|(b, t)| VbnBone {
                name: b.name.clone(),
                hash: b.bone_id,
                // TODO: Figure out why 0xFFFFFFF is used instead of 0xFFFFFFFF.
                parent_bone_index: u16::try_from(b.parent_bone_index).ok().map(Into::into),
                bone_type: b.bone_type,
                translation: t.translation.into(),
                rotation: t.rotation.into(),
                scale: t.scale.into(),
            })
            .collect(),
    }
}

fn material(m: &NudMaterial) -> Material {
    Material {
        shader_id: m.shader_id,
        unk1: 0,
        src_factor: m.src_factor,
        tex_count: m.textures.len() as u16,
        dst_factor: m.dst_factor,
        alpha_func: m.alpha_func,
        alpha_test_ref: m.alpha_test_ref,
        cull_mode: m.cull_mode,
        unk2: 0,
        unk3: 0,
        z_buffer_offset: 0,
        textures: m
            .textures
            .iter()
            .map(|t| MaterialTexture {
                hash: t.hash,
                unk1: [0; 3],
                map_mode: t.map_mode,
                wrap_mode_s: t.wrap_mode_s,
                wrap_mode_t: t.wrap_mode_t,
                min_filter: t.min_filter,
                mag_filter: t.mag_filter,
                mip_detail: t.mip_detail,
                unk2: 0,
                unk3: 0,
                unk4: 0,
            })
            .collect(),
        properties: m
            .properties
            .iter()
            .map(|p| {
                let size = if p.name == "NU_materialHash" {
                    0
                } else {
                    16 + p.values.len() as u32 * 4
                };
                MaterialProperty {
                    size,
                    name: p.name.clone(),
                    unk1: [0; 3],
                    value_count: p.values.len() as u8,
                    unk2: 0,
                    values: p.values.clone(),
                }
            })
            .collect(),
    }
}

fn bounding_sphere(sphere: Vec4) -> BoundingSphere {
    BoundingSphere {
        center: sphere.xyz().to_array(),
        radius: sphere.w,
    }
}

fn nut_textures(nut: &Nut) -> Result<Vec<ImageTexture>, CreateSurfaceError> {
    match nut {
        Nut::Ntwu(ntwu) => ntwu
            .textures
            .iter()
            .map(|t| Ok(ImageTexture::from_surface(t.gidx.hash, t.to_surface()?)))
            .collect(),
        Nut::Ntp3(ntp3) => ntp3
            .textures
            .iter()
            .map(|t| Ok(ImageTexture::from_surface(t.gidx.hash, t.to_surface()?)))
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
        shader_id: material.shader_id,
        src_factor: material.src_factor,
        dst_factor: material.dst_factor,
        alpha_func: material.alpha_func,
        alpha_test_ref: material.alpha_test_ref,
        cull_mode: material.cull_mode,
        textures: material
            .textures
            .iter()
            .map(|t| NudTexture {
                hash: t.hash,
                map_mode: t.map_mode,
                wrap_mode_s: t.wrap_mode_s,
                wrap_mode_t: t.wrap_mode_t,
                min_filter: t.min_filter,
                mag_filter: t.mag_filter,
                mip_detail: t.mip_detail,
            })
            .collect(),
        properties: material
            .properties
            .iter()
            .map(|p| NudProperty {
                name: p.name.clone(),
                values: p.values.clone(),
            })
            .collect(),
    }
}

impl ImageTexture {
    /// Create a view of all image data in this texture
    /// to use with encode or decode operations.
    pub fn to_surface(&self) -> Result<image_dds::Surface<&[u8]>, CreateSurfaceError> {
        Ok(image_dds::Surface {
            width: self.width,
            height: self.height,
            depth: 1,
            layers: 1,
            mipmaps: self.mipmap_count,
            image_format: self.image_format.try_into()?,
            data: &self.image_data,
        })
    }

    pub fn from_surface<T: AsRef<[u8]>>(hash_id: u32, surface: image_dds::Surface<T>) -> Self {
        Self {
            hash_id,
            width: surface.width,
            height: surface.height,
            mipmap_count: surface.mipmaps,
            layers: surface.layers,
            image_format: surface.image_format.into(),
            image_data: surface.data.as_ref().to_vec(),
        }
    }
}
