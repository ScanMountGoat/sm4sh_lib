use std::io::Cursor;

use bilge::prelude::*;
use binrw::{BinRead, BinReaderExt, BinResult, BinWrite, BinWriterExt, VecArgs};
use half::f16;

use sm4sh_lib::nud::{BoneType, ColorType, NormalType, UvColorFlags, UvType, VertexFlags};

// TODO: use glam types directly?
// TODO: Is it possible to rebuild the vertex buffers from this?
// TODO: Find a simpler representation after looking at more game data like pokken.
#[derive(Debug, PartialEq, Clone)]
pub struct Vertices {
    pub positions: Vec<[f32; 3]>,
    pub normals: Normals,
    pub bones: Bones,
    pub colors: Colors,
    // TODO: Move the vec inside the enum to guarantee the same type?
    pub uvs: Vec<Uvs>,
}

impl Vertices {
    // TODO: method that returns vertex flags
}

#[derive(Debug, PartialEq, Clone)]
pub enum Normals {
    None(Vec<f32>),
    NormalsFloat32(Vec<NormalsFloat32>),
    Unk2(Vec<NormalsUnk2>),
    NormalsTangentBitangentFloat32(Vec<NormalsTangentBitangentFloat32>),
    NormalsFloat16(Vec<NormalsFloat16>),
    NormalsTangentBitangentFloat16(Vec<NormalsTangentBitangentFloat16>),
}

impl Normals {
    fn normal_type(&self) -> NormalType {
        match self {
            Normals::None(_) => NormalType::None,
            Normals::NormalsFloat32(_) => NormalType::NormalsFloat32,
            Normals::Unk2(_) => NormalType::Unk2,
            Normals::NormalsTangentBitangentFloat32(_) => {
                NormalType::NormalsTangentBitangentFloat32
            }
            Normals::NormalsFloat16(_) => NormalType::NormalsFloat16,
            Normals::NormalsTangentBitangentFloat16(_) => {
                NormalType::NormalsTangentBitangentFloat16
            }
        }
    }
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsFloat32 {
    pub unk1: f32,
    pub normal: [f32; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct NormalsUnk2 {
    pub unk1: f32,
    pub normal: [f32; 4],
    // TODO: is this order correct?
    pub bitangent: [f32; 4],
    pub tangent: [f32; 4],
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
pub enum Bones {
    None,
    Float32(Vec<BonesFloat32>),
    Float16(Vec<BonesFloat16>),
    Byte(Vec<BonesByte>),
}

impl Bones {
    fn bone_type(&self) -> BoneType {
        match self {
            Bones::None => BoneType::None,
            Bones::Float32(_) => BoneType::Float32,
            Bones::Float16(_) => BoneType::Float16,
            Bones::Byte(_) => BoneType::Byte,
        }
    }
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesFloat32 {
    bone_indices: [u32; 4],
    bone_weights: [f32; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesFloat16 {
    bone_indices: [u16; 4],

    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    bone_weights: [f16; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct BonesByte {
    bone_indices: [u8; 4],
    bone_weights: [u8; 4], // TODO: unorm8?
}

#[derive(Debug, PartialEq, Clone)]
pub enum Uvs {
    Float16(Vec<UvsFloat16>),
    Float32(Vec<UvsFloat32>),
}

impl Uvs {
    fn uv_type(&self) -> UvType {
        match self {
            Uvs::Float16(_) => UvType::Float16,
            Uvs::Float32(_) => UvType::Float32,
        }
    }
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct UvsFloat16 {
    #[br(map = f16::from_bits)]
    #[bw(map = |x| x.to_bits())]
    u: f16,

    #[br(map = f16::from_bits)]
    #[bw(map = |x| x.to_bits())]
    v: f16,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct UvsFloat32 {
    u: f32,
    v: f32,
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
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct ColorFloat16 {
    #[br(map = |x: [u16; 4]| x.map(f16::from_bits))]
    #[bw(map = |x| x.map(f16::to_bits))]
    rgba: [f16; 4],
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct ColorByte {
    rgba: [u8; 4],
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
    vertex_flags: VertexFlags,
    uv_color_flags: UvColorFlags,
    count: u16,
) -> BinResult<Vertices> {
    let stride0 = buffer0_stride(vertex_flags, uv_color_flags);
    let stride1 = buffer1_stride(vertex_flags);

    // TODO: Is it better to do flags -> vec<Attribute> instead?
    if vertex_flags.bones() != BoneType::None {
        // buffer0: colors, uvs
        let mut offset0 = 0;

        let colors = read_colors(buffer0, uv_color_flags, offset0, stride0, count)?;
        offset0 += color_size(uv_color_flags);

        let mut uvs = Vec::new();
        for _ in 0..uv_color_flags.uv_count().value() {
            let uv = read_uvs(buffer0, uv_color_flags, offset0, stride0, count)?;
            offset0 += uvs_size(uv_color_flags);
            uvs.push(uv);
        }

        // buffer1: positions, vectors, bones,
        let mut offset1 = 0;

        let positions = read_positions(buffer1, offset1, stride1, count)?;
        offset1 += 12;

        let normals = read_normals(buffer1, vertex_flags, offset1, stride1, count)?;
        offset1 += normals_size(vertex_flags);

        let bones = read_bones(buffer1, vertex_flags, offset1, stride1, count)?;
        offset1 += bones_size(vertex_flags);

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

        let normals = read_normals(buffer0, vertex_flags, offset0, stride0, count)?;
        offset0 += normals_size(vertex_flags);

        let bones = read_bones(buffer0, vertex_flags, offset0, stride0, count)?;
        offset0 += bones_size(vertex_flags);

        let colors = read_colors(buffer0, uv_color_flags, offset0, stride0, count)?;
        offset0 += color_size(uv_color_flags);

        let mut uvs = Vec::new();
        for _ in 0..uv_color_flags.uv_count().value() {
            let uv = read_uvs(buffer0, uv_color_flags, offset0, stride0, count)?;
            offset0 += uvs_size(uv_color_flags);
            uvs.push(uv);
        }

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
) -> BinResult<(VertexFlags, UvColorFlags)> {
    let vertex_flags = VertexFlags::new(vertices.normals.normal_type(), vertices.bones.bone_type());
    let uv_color_flags = UvColorFlags::new(
        vertices.uvs[0].uv_type(), // TODO: what type if no uvs?
        vertices.colors.color_type(),
        u4::new(vertices.uvs.len().try_into().unwrap()),
    );

    let stride0 = buffer0_stride(vertex_flags, uv_color_flags);
    let stride1 = buffer1_stride(vertex_flags);

    if vertices.bones != Bones::None {
        // buffer0: colors, uvs
        let mut offset0 = buffer0.position();

        write_colors(buffer0, &vertices.colors, offset0, stride0)?;
        offset0 += color_size(uv_color_flags);

        for uv in &vertices.uvs {
            write_uvs(buffer0, uv, offset0, stride0)?;
            offset0 += uvs_size(uv_color_flags);
        }

        // buffer1: positions, vectors, bones,
        let mut offset1 = buffer1.position();

        write_positions(buffer1, &vertices.positions, offset1, stride1)?;
        offset1 += 12;

        write_normals(buffer1, &vertices.normals, offset1, stride1)?;
        offset1 += normals_size(vertex_flags);

        write_bones(buffer1, &vertices.bones, offset1, stride1)?;
        offset1 += bones_size(vertex_flags);
    } else {
        // buffer0: positions, vectors, bones, colors, uvs
        let mut offset0 = buffer0.position();

        write_positions(buffer0, &vertices.positions, offset0, stride0)?;
        offset0 += 12;

        write_normals(buffer0, &vertices.normals, offset0, stride0)?;
        offset0 += normals_size(vertex_flags);

        write_bones(buffer0, &vertices.bones, offset0, stride0)?;
        offset0 += bones_size(vertex_flags);

        write_colors(buffer0, &vertices.colors, offset0, stride0)?;
        offset0 += color_size(uv_color_flags);

        for uv in &vertices.uvs {
            write_uvs(buffer0, uv, offset0, stride0)?;
            offset0 += uvs_size(uv_color_flags);
        }
    }

    Ok((vertex_flags, uv_color_flags))
}

fn read_bones(
    buffer: &[u8],
    flags: VertexFlags,
    offset: u64,
    stride: u64,
    count: u16,
) -> BinResult<Bones> {
    match flags.bones() {
        BoneType::None => Ok(Bones::None),
        BoneType::Float32 => read_elements(buffer, stride, offset, count).map(Bones::Float32),
        BoneType::Float16 => read_elements(buffer, stride, offset, count).map(Bones::Float16),
        BoneType::Byte => read_elements(buffer, stride, offset, count).map(Bones::Byte),
    }
}

fn write_bones(
    buffer: &mut Cursor<Vec<u8>>,
    bones: &Bones,
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    match bones {
        Bones::None => Ok(()),
        Bones::Float32(elements) => write_elements(buffer, elements, stride, offset),
        Bones::Float16(elements) => write_elements(buffer, elements, stride, offset),
        Bones::Byte(elements) => write_elements(buffer, elements, stride, offset),
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
        NormalType::Unk2 => read_elements(buffer, stride, offset, count).map(Normals::Unk2),
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
        Normals::Unk2(elements) => write_elements(buffer, elements, stride, offset),
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
    flags: UvColorFlags,
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
    flags: UvColorFlags,
    offset: u64,
    stride: u64,
    count: u16,
) -> BinResult<Uvs> {
    match flags.uvs() {
        UvType::Float16 => read_elements(buffer, stride, offset, count).map(Uvs::Float16),
        UvType::Float32 => read_elements(buffer, stride, offset, count).map(Uvs::Float32),
    }
}

fn write_uvs(buffer: &mut Cursor<Vec<u8>>, uvs: &Uvs, offset: u64, stride: u64) -> BinResult<()> {
    match uvs {
        Uvs::Float16(elements) => write_elements(buffer, elements, stride, offset),
        Uvs::Float32(elements) => write_elements(buffer, elements, stride, offset),
    }
}

fn read_positions(buffer: &[u8], offset: u64, stride: u64, count: u16) -> BinResult<Vec<[f32; 3]>> {
    read_elements(buffer, stride, offset, count)
}

fn write_positions(
    buffer: &mut Cursor<Vec<u8>>,
    positions: &[[f32; 3]],
    offset: u64,
    stride: u64,
) -> BinResult<()> {
    write_elements(buffer, positions, stride, offset)
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
fn buffer0_stride(vertex: VertexFlags, uv_color: UvColorFlags) -> u64 {
    if vertex.bones() != BoneType::None {
        uvs_color_size(uv_color)
    } else {
        vertex_size(vertex) + uvs_color_size(uv_color)
    }
}

fn buffer1_stride(vertex: VertexFlags) -> u64 {
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
        NormalType::Unk2 => 13 * 4,
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

fn uvs_color_size(flags: UvColorFlags) -> u64 {
    uvs_size(flags) * flags.uv_count().value() as u64 + color_size(flags)
}

fn uvs_size(flags: UvColorFlags) -> u64 {
    match flags.uvs() {
        UvType::Float16 => 2 * 2,
        UvType::Float32 => 2 * 4,
    }
}

fn color_size(flags: UvColorFlags) -> u64 {
    match flags.colors() {
        ColorType::None => 0,
        ColorType::Byte => 4,
        ColorType::Float16 => 4 * 2,
    }
}

// TODO: Attribute with buffer index, relative offset, data type?
// flags -> attributes -> position, uv, color, normal, bone data?
// TODO: Add tests for rebuilding vertex data

#[cfg(test)]
mod tests {
    use super::*;

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

        let vertex_flags = VertexFlags::new(NormalType::NormalsFloat16, BoneType::None);
        let uv_color_flags = UvColorFlags::new(UvType::Float16, ColorType::Byte, u4::new(2));

        let vertices = read_vertices(&buffer0, &[], vertex_flags, uv_color_flags, 2).unwrap();

        // Check read.
        assert_eq!(
            Vertices {
                positions: vec![
                    [0.9634427, 11.351391, 1.9046955],
                    [1.1407516, 11.469169, 1.8017474]
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
                bones: Bones::None,
                colors: Colors::Byte(vec![
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    },
                    ColorByte {
                        rgba: [127, 127, 127, 127]
                    }
                ]),
                uvs: vec![
                    Uvs::Float16(vec![
                        UvsFloat16 {
                            u: f16::from_f32(0.68359375),
                            v: f16::from_f32(0.8857422)
                        },
                        UvsFloat16 {
                            u: f16::from_f32(0.80859375),
                            v: f16::from_f32(0.8173828)
                        }
                    ]),
                    Uvs::Float16(vec![
                        UvsFloat16 {
                            u: f16::from_f32(0.59277344),
                            v: f16::from_f32(0.8339844),
                        },
                        UvsFloat16 {
                            u: f16::from_f32(0.6928711),
                            v: f16::from_f32(0.7651367)
                        }
                    ])
                ]
            },
            vertices
        );

        // Check write.
        let mut new_buffer0 = Cursor::new(Vec::new());
        let mut new_buffer1 = Cursor::new(Vec::new());
        assert_eq!(
            (vertex_flags, uv_color_flags),
            write_vertices(&vertices, &mut new_buffer0, &mut new_buffer1).unwrap()
        );
        assert_eq!(buffer0, &new_buffer0.into_inner()[..]);
        assert!(new_buffer1.into_inner().is_empty())
    }
}
