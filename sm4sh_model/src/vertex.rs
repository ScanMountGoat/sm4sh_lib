use std::io::Cursor;

use bilge::prelude::*;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, VecArgs};
use glam::{vec2, Vec2, Vec3, Vec4};
use half::f16;

use sm4sh_lib::nud::{BoneType, ColorType, NormalType, UvType, VertexFlags};

// TODO: Is it possible to rebuild the vertex buffers from this?
// TODO: Find a simpler representation after looking at more game data like pokken.
#[derive(Debug, PartialEq, Clone)]
pub struct Vertices {
    pub positions: Vec<Vec3>,
    pub normals: Normals,
    pub bones: Option<Bones>,
    pub colors: Colors,
    pub uvs: Uvs,
}

impl Vertices {
    fn bone_type(&self) -> BoneType {
        match self.bones.as_ref().map(|b| b.element_type) {
            None => BoneType::None,
            Some(BoneElementType::Float32) => BoneType::Float32,
            Some(BoneElementType::Float16) => BoneType::Float16,
            Some(BoneElementType::Byte) => BoneType::Byte,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Normals {
    None(Vec<f32>),
    NormalsFloat32(Vec<NormalsFloat32>),
    NormalsTangentBitangentFloat32(Vec<NormalsTangentBitangentFloat32>),
    NormalsFloat16(Vec<NormalsFloat16>),
    NormalsTangentBitangentFloat16(Vec<NormalsTangentBitangentFloat16>),
}

impl Normals {
    fn normal_type(&self) -> NormalType {
        match self {
            Normals::None(_) => NormalType::None,
            Normals::NormalsFloat32(_) => NormalType::NormalsFloat32,
            Normals::NormalsTangentBitangentFloat32(_) => {
                NormalType::NormalsTangentBitangentFloat32
            }
            Normals::NormalsFloat16(_) => NormalType::NormalsFloat16,
            Normals::NormalsTangentBitangentFloat16(_) => {
                NormalType::NormalsTangentBitangentFloat16
            }
        }
    }

    pub fn normals(&self) -> Option<Vec<Vec4>> {
        match self {
            Normals::None(_) => None,
            Normals::NormalsFloat32(items) => Some(items.iter().map(|i| i.normal.into()).collect()),
            Normals::NormalsTangentBitangentFloat32(items) => {
                Some(items.iter().map(|i| i.normal.into()).collect())
            }
            Normals::NormalsFloat16(items) => Some(
                items
                    .iter()
                    .map(|i| i.normal.map(|f| f.to_f32()).into())
                    .collect(),
            ),
            Normals::NormalsTangentBitangentFloat16(items) => Some(
                items
                    .iter()
                    .map(|i| i.normal.map(|f| f.to_f32()).into())
                    .collect(),
            ),
        }
    }

    // TODO: "constructor" for each variant using attribute arrays?
    // TODO: Just redo the variants to work like this instead?
    // structs <-> attribute arrays
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsFloat32 {
    pub unk1: f32,
    pub normal: [f32; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsTangentBitangentFloat32 {
    pub unk1: f32,
    pub normal: [f32; 4],
    // TODO: is this order correct?
    pub bitangent: [f32; 4],
    pub tangent: [f32; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsFloat16 {
    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub normal: [f16; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsTangentBitangentFloat16 {
    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub normal: [f16; 4],

    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub bitangent: [f16; 4],

    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub tangent: [f16; 4],
}

#[derive(Debug, PartialEq, Clone)]
pub struct Bones {
    pub bone_indices: Vec<[u32; 4]>,
    pub weights: Vec<Vec4>,
    pub element_type: BoneElementType,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BoneElementType {
    Float32,
    Float16,
    Byte,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesFloat32 {
    pub bone_indices: [u32; 4],
    pub bone_weights: [f32; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesFloat16 {
    pub bone_indices: [u16; 4],

    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub bone_weights: [f16; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesByte {
    pub bone_indices: [u8; 4],
    pub bone_weights: [u8; 4], // TODO: unorm8?
}

#[derive(Debug, PartialEq, Clone)]
pub enum Uvs {
    Float16(Vec<Vec<UvFloat16>>),
    Float32(Vec<Vec<UvFloat32>>),
}

impl Uvs {
    fn uv_type(&self) -> UvType {
        match self {
            Uvs::Float16(_) => UvType::Float16,
            Uvs::Float32(_) => UvType::Float32,
        }
    }

    fn len(&self) -> usize {
        match self {
            Uvs::Float16(items) => items.len(),
            Uvs::Float32(items) => items.len(),
        }
    }

    pub fn uvs(&self) -> Vec<Vec<Vec2>> {
        match self {
            Uvs::Float16(items) => items
                .iter()
                .map(|i| i.iter().map(|i| vec2(i.u.to_f32(), i.v.to_f32())).collect())
                .collect(),
            Uvs::Float32(items) => items
                .iter()
                .map(|i| i.iter().map(|i| vec2(i.u, i.v)).collect())
                .collect(),
        }
    }
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct UvFloat16 {
    #[br(map = f16::from_bits)]
    #[bw(map = |x| x.to_bits())]
    pub u: f16,

    #[br(map = f16::from_bits)]
    #[bw(map = |x| x.to_bits())]
    pub v: f16,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct UvFloat32 {
    pub u: f32,
    pub v: f32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Colors {
    None,
    Byte(Vec<ColorByte>),
    Float16(Vec<ColorFloat16>),
}

impl Colors {
    fn color_type(&self) -> ColorType {
        match self {
            Colors::None => ColorType::None,
            Colors::Byte(_) => ColorType::Byte,
            Colors::Float16(_) => ColorType::Float16,
        }
    }

    pub fn colors(&self) -> Option<Vec<Vec4>> {
        match self {
            Colors::None => None,
            Colors::Byte(items) => Some(
                items
                    .iter()
                    .map(|i| i.rgba.map(|u| u as f32 / 255.0).into())
                    .collect(),
            ),
            Colors::Float16(items) => Some(
                items
                    .iter()
                    .map(|i| i.rgba.map(|f| f.to_f32()).into())
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct ColorFloat16 {
    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    pub rgba: [f16; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct ColorByte {
    pub rgba: [u8; 4],
}

pub fn read_vertex_indices(buffer: &[u8], count: u16) -> BinResult<Vec<u16>> {
    Cursor::new(buffer).read_be_args(VecArgs {
        count: count as usize,
        inner: (),
    })
}

pub fn write_vertex_indices(buffer: &mut Cursor<Vec<u8>>, indices: &[u16]) -> BinResult<()> {
    buffer.write_be(&indices)
}

pub fn read_vertices(
    buffer0: &[u8],
    buffer1: &[u8],
    flags: VertexFlags,
    count: u16,
) -> BinResult<Vertices> {
    let stride0 = buffer0_stride(flags);
    let stride1 = buffer1_stride(flags);

    // TODO: Is it better to do flags -> vec<Attribute> instead?
    if flags.bones() != BoneType::None {
        // buffer0: colors, uvs
        let mut offset0 = 0;

        let colors = read_colors(buffer0, flags, offset0, stride0, count)?;
        offset0 += color_size(flags);

        let uvs = read_uvs(buffer0, flags, &mut offset0, stride0, count)?;

        // buffer1: positions, vectors, bones,
        let mut offset1 = 0;

        let positions = read_positions(buffer1, offset1, stride1, count)?;
        offset1 += 12;

        let normals = read_normals(buffer1, flags, offset1, stride1, count)?;
        offset1 += normals_size(flags);

        let bones = read_bones(buffer1, flags, offset1, stride1, count)?;
        offset1 += bones_size(flags);

        Ok(Vertices {
            positions,
            normals,
            bones,
            colors,
            uvs,
        })
    } else {
        // buffer0: positions, vectors, bones, colors, uvs
        let mut offset0 = 0;

        let positions = read_positions(buffer0, offset0, stride0, count)?;
        offset0 += 12;

        let normals = read_normals(buffer0, flags, offset0, stride0, count)?;
        offset0 += normals_size(flags);

        let bones = read_bones(buffer0, flags, offset0, stride0, count)?;
        offset0 += bones_size(flags);

        let colors = read_colors(buffer0, flags, offset0, stride0, count)?;
        offset0 += color_size(flags);

        let uvs = read_uvs(buffer0, flags, &mut offset0, stride0, count)?;

        Ok(Vertices {
            positions,
            normals,
            bones,
            colors,
            uvs,
        })
    }
}

pub fn write_vertices(
    vertices: &Vertices,
    buffer0: &mut Cursor<Vec<u8>>,
    buffer1: &mut Cursor<Vec<u8>>,
) -> BinResult<VertexFlags> {
    let flags = VertexFlags::new(
        vertices.uvs.uv_type(),
        vertices.colors.color_type(),
        u4::new(vertices.uvs.len().try_into().unwrap()),
        vertices.normals.normal_type(),
        vertices.bone_type(),
    );

    let stride0 = buffer0_stride(flags);
    let stride1 = buffer1_stride(flags);

    if vertices.bones.is_some() {
        // buffer0: colors, uvs
        let mut offset0 = buffer0.position();

        write_colors(buffer0, &vertices.colors, offset0, stride0)?;
        offset0 += color_size(flags);

        write_uvs(buffer0, &vertices.uvs, &mut offset0, stride0)?;

        // buffer1: positions, vectors, bones,
        let mut offset1 = buffer1.position();

        write_positions(buffer1, &vertices.positions, offset1, stride1)?;
        offset1 += 12;

        write_normals(buffer1, &vertices.normals, offset1, stride1)?;
        offset1 += normals_size(flags);

        if let Some(bones) = &vertices.bones {
            write_bones(buffer1, bones, offset1, stride1)?;
            offset1 += bones_size(flags);
        }
    } else {
        // buffer0: positions, vectors, bones, colors, uvs
        let mut offset0 = buffer0.position();

        write_positions(buffer0, &vertices.positions, offset0, stride0)?;
        offset0 += 12;

        write_normals(buffer0, &vertices.normals, offset0, stride0)?;
        offset0 += normals_size(flags);

        if let Some(bones) = &vertices.bones {
            // TODO: Is this code ever reached?
            write_bones(buffer0, bones, offset0, stride0)?;
            offset0 += bones_size(flags);
        }

        write_colors(buffer0, &vertices.colors, offset0, stride0)?;
        offset0 += color_size(flags);

        write_uvs(buffer0, &vertices.uvs, &mut offset0, stride0)?;
    }

    Ok(flags)
}

fn read_bones(
    buffer: &[u8],
    flags: VertexFlags,
    offset: u64,
    stride: u64,
    count: u16,
) -> BinResult<Option<Bones>> {
    match flags.bones() {
        BoneType::None => Ok(None),
        BoneType::Float32 => {
            let elements: Vec<BonesFloat32> = read_elements(buffer, stride, offset, count)?;
            Ok(Some(Bones {
                bone_indices: elements.iter().map(|i| i.bone_indices).collect(),
                weights: elements.iter().map(|i| i.bone_weights.into()).collect(),
                element_type: BoneElementType::Float32,
            }))
        }
        BoneType::Float16 => {
            let elements: Vec<BonesFloat16> = read_elements(buffer, stride, offset, count)?;
            Ok(Some(Bones {
                bone_indices: elements
                    .iter()
                    .map(|i| i.bone_indices.map(Into::into))
                    .collect(),
                weights: elements
                    .iter()
                    .map(|i| i.bone_weights.map(f16::to_f32).into())
                    .collect(),
                element_type: BoneElementType::Float16,
            }))
        }
        BoneType::Byte => {
            let elements: Vec<BonesByte> = read_elements(buffer, stride, offset, count)?;
            Ok(Some(Bones {
                bone_indices: elements
                    .iter()
                    .map(|i| i.bone_indices.map(Into::into))
                    .collect(),
                weights: elements
                    .iter()
                    .map(|i| i.bone_weights.map(|u| (u as f32) / 255.0).into())
                    .collect(),
                element_type: BoneElementType::Byte,
            }))
        }
    }
}

fn write_bones(
    buffer: &mut Cursor<Vec<u8>>,
    bones: &Bones,
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    match bones.element_type {
        BoneElementType::Float32 => {
            let elements: Vec<_> = bones
                .bone_indices
                .iter()
                .zip(&bones.weights)
                .map(|(i, w)| BonesFloat32 {
                    bone_indices: *i,
                    bone_weights: w.to_array(),
                })
                .collect();

            write_elements(buffer, &elements, stride, offset)
        }
        BoneElementType::Float16 => {
            let elements: Vec<_> = bones
                .bone_indices
                .iter()
                .zip(&bones.weights)
                .map(|(i, w)| BonesFloat16 {
                    bone_indices: i.map(|u| u as u16),
                    bone_weights: w.to_array().map(f16::from_f32),
                })
                .collect();

            write_elements(buffer, &elements, stride, offset)
        }
        BoneElementType::Byte => {
            let elements: Vec<_> = bones
                .bone_indices
                .iter()
                .zip(&bones.weights)
                .map(|(i, w)| BonesByte {
                    bone_indices: i.map(|u| u as u8),
                    bone_weights: w.to_array().map(|f| (f * 255.0) as u8),
                })
                .collect();

            write_elements(buffer, &elements, stride, offset)
        }
    }
}

fn read_normals(
    buffer: &[u8],
    flags: VertexFlags,
    offset: u64,
    stride: u64,
    count: u16,
) -> BinResult<Normals> {
    match flags.normals() {
        NormalType::None => read_elements(buffer, stride, offset, count).map(Normals::None),
        NormalType::NormalsFloat32 => {
            read_elements(buffer, stride, offset, count).map(Normals::NormalsFloat32)
        }
        NormalType::NormalsTangentBitangentFloat32 => read_elements(buffer, stride, offset, count)
            .map(Normals::NormalsTangentBitangentFloat32),
        NormalType::NormalsFloat16 => {
            read_elements(buffer, stride, offset, count).map(Normals::NormalsFloat16)
        }
        NormalType::NormalsTangentBitangentFloat16 => read_elements(buffer, stride, offset, count)
            .map(Normals::NormalsTangentBitangentFloat16),
    }
}

fn write_normals(
    buffer: &mut Cursor<Vec<u8>>,
    normals: &Normals,
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    match normals {
        Normals::None(elements) => write_elements(buffer, elements, stride, offset),
        Normals::NormalsFloat32(elements) => write_elements(buffer, elements, stride, offset),
        Normals::NormalsTangentBitangentFloat32(elements) => {
            write_elements(buffer, elements, stride, offset)
        }
        Normals::NormalsFloat16(elements) => write_elements(buffer, elements, stride, offset),
        Normals::NormalsTangentBitangentFloat16(elements) => {
            write_elements(buffer, elements, stride, offset)
        }
    }
}

fn read_colors(
    buffer: &[u8],
    flags: VertexFlags,
    offset: u64,
    stride: u64,
    count: u16,
) -> BinResult<Colors> {
    match flags.colors() {
        ColorType::None => Ok(Colors::None),
        ColorType::Byte => read_elements(buffer, stride, offset, count).map(Colors::Byte),
        ColorType::Float16 => read_elements(buffer, stride, offset, count).map(Colors::Float16),
    }
}

fn write_colors(
    buffer: &mut Cursor<Vec<u8>>,
    colors: &Colors,
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    match colors {
        Colors::None => Ok(()),
        Colors::Byte(elements) => write_elements(buffer, elements, stride, offset),
        Colors::Float16(elements) => write_elements(buffer, elements, stride, offset),
    }
}

fn read_uvs(
    buffer: &[u8],
    flags: VertexFlags,
    offset: &mut u64,
    stride: u64,
    count: u16,
) -> BinResult<Uvs> {
    match flags.uvs() {
        UvType::Float16 => {
            let mut layers = Vec::new();
            for _ in 0..flags.uv_count().value() {
                let layer = read_elements(buffer, stride, *offset, count)?;
                *offset += uvs_size(flags.uvs());

                layers.push(layer);
            }
            Ok(Uvs::Float16(layers))
        }
        UvType::Float32 => {
            let mut layers = Vec::new();
            for _ in 0..flags.uv_count().value() {
                let layer = read_elements(buffer, stride, *offset, count)?;
                *offset += uvs_size(flags.uvs());

                layers.push(layer);
            }
            Ok(Uvs::Float32(layers))
        }
    }
}

fn write_uvs(
    buffer: &mut Cursor<Vec<u8>>,
    uvs: &Uvs,
    offset: &mut u64,
    stride: u64,
) -> BinResult<()> {
    match uvs {
        Uvs::Float16(elements) => {
            for layer in elements {
                write_elements(buffer, layer, stride, *offset)?;
                *offset += uvs_size(uvs.uv_type());
            }
        }
        Uvs::Float32(elements) => {
            for layer in elements {
                write_elements(buffer, layer, stride, *offset)?;
                *offset += uvs_size(uvs.uv_type());
            }
        }
    }
    Ok(())
}

fn read_positions(buffer: &[u8], offset: u64, stride: u64, count: u16) -> BinResult<Vec<Vec3>> {
    Ok(read_elements::<[f32; 3]>(buffer, stride, offset, count)?
        .into_iter()
        .map(Into::into)
        .collect())
}

fn write_positions(
    buffer: &mut Cursor<Vec<u8>>,
    positions: &[Vec3],
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    let elements: Vec<_> = positions.iter().map(|v| v.to_array()).collect();
    write_elements(buffer, &elements, stride, offset)
}

fn read_elements<T>(buffer: &[u8], stride: u64, offset: u64, count: u16) -> BinResult<Vec<T>>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let mut reader = Cursor::new(buffer);

    let mut elements = Vec::new();
    for i in 0..count {
        reader.set_position(offset + i as u64 * stride);
        let element: T = reader.read_be()?;
        elements.push(element);
    }

    Ok(elements)
}

fn write_elements<T>(
    buffer: &mut Cursor<Vec<u8>>,
    elements: &[T],
    stride: u64,
    offset: u64,
) -> BinResult<()>
where
    for<'a> T: BinWrite<Args<'a> = ()>,
{
    for (i, element) in elements.iter().enumerate() {
        buffer.set_position(offset + i as u64 * stride);
        buffer.write_be(element)?;
    }

    Ok(())
}

// TODO: Is it better to just create attributes instead?
pub fn buffer0_stride(flags: VertexFlags) -> u64 {
    if flags.bones() != BoneType::None {
        uvs_color_size(flags)
    } else {
        vertex_size(flags) + uvs_color_size(flags)
    }
}

pub fn buffer1_stride(vertex: VertexFlags) -> u64 {
    if vertex.bones() != BoneType::None {
        vertex_size(vertex)
    } else {
        0
    }
}

fn vertex_size(flags: VertexFlags) -> u64 {
    let position_size = 3 * 4;
    position_size + normals_size(flags) + bones_size(flags)
}

fn normals_size(flags: VertexFlags) -> u64 {
    match flags.normals() {
        NormalType::None => 4,
        NormalType::NormalsFloat32 => 5 * 4,
        NormalType::NormalsTangentBitangentFloat32 => 13 * 4,
        NormalType::NormalsFloat16 => 4 * 2,
        NormalType::NormalsTangentBitangentFloat16 => 12 * 2,
    }
}

fn bones_size(flags: VertexFlags) -> u64 {
    match flags.bones() {
        BoneType::None => 0,
        BoneType::Float32 => 8 * 4,
        BoneType::Float16 => 8 * 2,
        BoneType::Byte => 8,
    }
}

fn uvs_color_size(flags: VertexFlags) -> u64 {
    uvs_size(flags.uvs()) * flags.uv_count().value() as u64 + color_size(flags)
}

fn uvs_size(flags: UvType) -> u64 {
    match flags {
        UvType::Float16 => 2 * 2,
        UvType::Float32 => 2 * 4,
    }
}

fn color_size(flags: VertexFlags) -> u64 {
    match flags.colors() {
        ColorType::None => 0,
        ColorType::Byte => 4,
        ColorType::Float16 => 4 * 2,
    }
}

pub fn triangle_strip_to_list(indices: &[u16]) -> Vec<u16> {
    let mut new_indices = Vec::new();

    let mut index = 0;
    for i in 0..indices.len() - 2 {
        let face = &indices[i..i + 3];

        // TODO: Skip degenerate triangles with zero area (repeated indices)..

        // Restart primitive assembly if the index is -1.
        // https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#drawing
        if face.contains(&u16::MAX) {
            index = 0;
            continue;
        } else {
            // Strip indices 0 1 2 3 4 generate triangles (0 1 2) (2 1 3) (2 3 4).
            if index % 2 == 0 {
                new_indices.extend([face[0], face[1], face[2]]);
            } else {
                new_indices.extend([face[1], face[0], face[2]]);
            }

            index += 1;
        }
    }

    new_indices
}

// TODO: Attribute with buffer index, relative offset, data type?
// flags -> attributes -> position, uv, color, normal, bone data?
// TODO: Add tests for rebuilding vertex data

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{vec3, vec4};
    use hexlit::hex;

    // TODO: Verify each type in game with renderdoc.
    // TODO: Add one test for each unique flags combination?

    #[test]
    fn read_write_vertex_indices_mario_face() {
        // data/fighter/mario/model/body/c00/model.nud, Mario_FaceN_VIS_O_OBJ, 0
        let buffer = hex!(00000001 00020000 00020003);

        let indices = read_vertex_indices(&buffer, 6).unwrap();
        assert_eq!(vec![0, 1, 2, 0, 2, 3], indices);

        let mut new_buffer = Cursor::new(Vec::new());
        write_vertex_indices(&mut new_buffer, &indices).unwrap();
        assert_eq!(buffer, &new_buffer.into_inner()[..]);
    }

    #[test]
    fn read_write_vertices_mario_eye() {
        // data/fighter/mario/model/body/c00/model.nud, Mario_Eye_VIS_O_OBJ, 0
        let buffer0 = hex!(
            // vertex 0
            3f76a42e 41359f4c 3ff3cd10 // position
            3772b426 3ac53c00          // normal
            7f7f7f7f                   // color
            39783b16                   // uv0
            38be3aac                   // uv1
            // vertex 1
            3f920426 413781b7 3fe69fa9 // position
            3932b180 0x39ec3c00        // normal
            7f7f7f7f                   // color
            3a783a8a                   // uv0
            398b3a1f                   // uv1
        );

        let vertex_flags = VertexFlags::new(
            UvType::Float16,
            ColorType::Byte,
            u4::new(2),
            NormalType::NormalsFloat16,
            BoneType::None,
        );
        let vertices = read_vertices(&buffer0, &[], vertex_flags, 2).unwrap();

        // Check read.
        assert_eq!(
            Vertices {
                positions: vec![
                    vec3(0.9634427, 11.351391, 1.9046955),
                    vec3(1.1407516, 11.469169, 1.8017474)
                ],
                normals: Normals::NormalsFloat16(vec![
                    NormalsFloat16 {
                        normal: [
                            f16::from_f32(0.46533203),
                            f16::from_f32(-0.25927734),
                            f16::from_f32(0.8461914),
                            f16::from_f32(1.0)
                        ]
                    },
                    NormalsFloat16 {
                        normal: [
                            f16::from_f32(0.64941406),
                            f16::from_f32(-0.171875),
                            f16::from_f32(0.7402344),
                            f16::from_f32(1.0)
                        ]
                    }
                ]),
                bones: None,
                colors: Colors::Byte(vec![
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    },
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    }
                ]),
                uvs: Uvs::Float16(vec![
                    vec![
                        UvFloat16 {
                            u: f16::from_f32(0.68359375),
                            v: f16::from_f32(0.8857422)
                        },
                        UvFloat16 {
                            u: f16::from_f32(0.80859375),
                            v: f16::from_f32(0.8173828)
                        }
                    ],
                    vec![
                        UvFloat16 {
                            u: f16::from_f32(0.59277344),
                            v: f16::from_f32(0.8339844),
                        },
                        UvFloat16 {
                            u: f16::from_f32(0.6928711),
                            v: f16::from_f32(0.7651367)
                        }
                    ]
                ])
            },
            vertices
        );

        // Check write.
        let mut new_buffer0 = Cursor::new(Vec::new());
        let mut new_buffer1 = Cursor::new(Vec::new());
        assert_eq!(
            vertex_flags,
            write_vertices(&vertices, &mut new_buffer0, &mut new_buffer1).unwrap()
        );
        assert_eq!(buffer0, &new_buffer0.into_inner()[..]);
        assert!(new_buffer1.into_inner().is_empty())
    }

    #[test]
    fn read_write_vertices_mario_body() {
        // data/fighter/mario/model/body/c00/model.nud, Gamemodel, 2
        let buffer0 = hex!(
            // vertex 0
            7F7F7F7F 389F356B
            // vertex 1
            7F7F7F7F 38D13588
        );

        let buffer1 = hex!(
            // vertex 0
            3F064FA1 411D7004 BF398361 30FC3BD2 B0843C00 B9E632BA 39223C00 B9429C2D BA073C00 0C150202 B24D0000
            // vertex 1
            0x3ED52310 411D504A BF671058 342B39D2 B9133C00 B507397B 39413C00 BB4DA737 B6843C00 0C150202 B24D0000
        );

        let flags = VertexFlags::new(
            UvType::Float16,
            ColorType::Byte,
            u4::new(1),
            NormalType::NormalsTangentBitangentFloat16,
            BoneType::Byte,
        );

        let vertices = read_vertices(&buffer0, &buffer1, flags, 2).unwrap();

        // Check read.
        assert_eq!(
            Vertices {
                positions: vec![
                    vec3(0.52465254, 9.839848, -0.72466093),
                    vec3(0.41628313, 9.832102, -0.90259314)
                ],
                normals: Normals::NormalsTangentBitangentFloat16(vec![
                    NormalsTangentBitangentFloat16 {
                        normal: [
                            f16::from_f32(0.15576172),
                            f16::from_f32(0.97753906),
                            f16::from_f32(-0.14111328),
                            f16::from_f32(1.0)
                        ],
                        bitangent: [
                            f16::from_f32(-0.7373047),
                            f16::from_f32(0.21020508),
                            f16::from_f32(0.64160156),
                            f16::from_f32(1.0)
                        ],
                        tangent: [
                            f16::from_f32(-0.65722656),
                            f16::from_f32(-0.0040779114),
                            f16::from_f32(-0.75341797),
                            f16::from_f32(1.0)
                        ]
                    },
                    NormalsTangentBitangentFloat16 {
                        normal: [
                            f16::from_f32(0.26049805),
                            f16::from_f32(0.72753906),
                            f16::from_f32(-0.63427734),
                            f16::from_f32(1.0)
                        ],
                        bitangent: [
                            f16::from_f32(-0.31420898),
                            f16::from_f32(0.6850586),
                            f16::from_f32(0.6567383),
                            f16::from_f32(1.0)
                        ],
                        tangent: [
                            f16::from_f32(-0.91259766),
                            f16::from_f32(-0.028182983),
                            f16::from_f32(-0.40722656),
                            f16::from_f32(1.0)
                        ]
                    }
                ]),
                bones: Some(Bones {
                    bone_indices: vec![[12, 21, 2, 2], [12, 21, 2, 2]],
                    weights: vec![
                        vec4(0.69803923, 0.3019608, 0.0, 0.0),
                        vec4(0.69803923, 0.3019608, 0.0, 0.0)
                    ],
                    element_type: BoneElementType::Byte
                }),
                colors: Colors::Byte(vec![
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    },
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    }
                ]),
                uvs: Uvs::Float16(vec![vec![
                    UvFloat16 {
                        u: f16::from_f32(0.5776367),
                        v: f16::from_f32(0.33862305)
                    },
                    UvFloat16 {
                        u: f16::from_f32(0.6020508),
                        v: f16::from_f32(0.34570313)
                    }
                ]])
            },
            vertices
        );

        // Check write.
        let mut new_buffer0 = Cursor::new(Vec::new());
        let mut new_buffer1 = Cursor::new(Vec::new());
        assert_eq!(
            flags,
            write_vertices(&vertices, &mut new_buffer0, &mut new_buffer1).unwrap()
        );
        assert_eq!(buffer0, &new_buffer0.into_inner()[..]);
        assert_eq!(buffer1, &new_buffer1.into_inner()[..]);
    }

    #[test]
    fn triangle_strip_to_list_basic() {
        assert_eq!(
            vec![0, 1, 2, 2, 1, 3, 2, 3, 4],
            triangle_strip_to_list(&[0, 1, 2, 3, 4])
        );
    }

    #[test]
    fn triangle_strip_to_list_restart() {
        assert_eq!(
            vec![0, 1, 2, 2, 3, 4, 4, 3, 5],
            triangle_strip_to_list(&[0, 1, 2, u16::MAX, 2, 3, 4, 5])
        );
    }
}
