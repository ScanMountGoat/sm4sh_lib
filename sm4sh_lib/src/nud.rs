use std::{
    cell::RefCell,
    io::{SeekFrom, Write},
    rc::Rc,
};

use bilge::prelude::*;
use binrw::{BinRead, BinWrite, binread, helpers::until};
use xc3_write::{
    Xc3Write, Xc3WriteOffsets,
    strings::{StringSection, WriteOptions},
};

use crate::{
    arbitrary_bilge_impl, parse_opt_ptr32, parse_ptr32_count, parse_string_ptr32,
    xc3_write_binwrite_impl,
};

// TODO: little endian for NDWD?
// TODO: Better naming
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"NDP3"))]
#[xc3(magic(b"NDP3"))]
pub struct Nud {
    #[xc3(shared_offset)]
    pub file_size: u32,
    pub version: u16,
    pub mesh_group_count: u16,
    /// The smallest bone index from vertex skinning or [MeshGroup] bone parenting.
    pub bone_start_index: u16,
    /// The largest bone index from vertex skinning or [MeshGroup] bone parenting.
    pub bone_end_index: u16,
    // TODO: update these in 2nd pass with xc3write?
    // TODO: Just make this an offset to combined vec<u8>?
    #[xc3(shared_offset)]
    pub indices_offset: u32, // vertex indices relative to 0x30?
    pub indices_size: u32,
    pub vertex_buffer0_size: u32,
    pub vertex_buffer1_size: u32,

    pub bounding_sphere: BoundingSphere,

    // TODO: Strings start at sum of above + header size (0x30)?
    // TODO: Separate header type with methods for these offsets?
    #[br(temp, calc = indices_offset + indices_size + vertex_buffer0_size + vertex_buffer1_size + 48)]
    strings_offset: u32,

    #[br(args { count: mesh_group_count as usize, inner: strings_offset })]
    pub mesh_groups: Vec<MeshGroup>,

    // TODO: Find a cleaner way to delay writing these buffers
    // TODO: Is there any alignment between buffers?
    #[br(seek_before = SeekFrom::Start(indices_offset as u64 + 48))]
    #[br(count = indices_size)]
    #[xc3(save_position(false))]
    pub index_buffer: Vec<u8>,

    #[br(seek_before = SeekFrom::Start((indices_offset + indices_size) as u64 + 48))]
    #[br(count = vertex_buffer0_size)]
    #[xc3(save_position(false))]
    pub vertex_buffer0: Vec<u8>,

    // TODO: What determines which buffer an attribute is part of?
    #[br(seek_before = SeekFrom::Start((indices_offset + indices_size + vertex_buffer0_size) as u64 + 48))]
    #[br(count = vertex_buffer1_size)]
    #[xc3(save_position(false))]
    pub vertex_buffer1: Vec<u8>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(strings_offset: u32))]
pub struct MeshGroup {
    pub bounding_sphere: BoundingSphere,
    pub center: [f32; 3],
    pub sort_bias: f32,
    #[br(parse_with = parse_string_ptr32, offset = strings_offset as u64)]
    #[xc3(offset(u32), align(16))]
    pub name: String,
    pub unk1: u16,
    pub bone_flags: BoneFlags,
    pub parent_bone_index: i16,
    pub mesh_count: u16,

    #[br(parse_with = parse_ptr32_count(mesh_count as usize))]
    #[br(args { inner: strings_offset })]
    #[xc3(offset(u32))]
    pub meshes: Vec<Mesh>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum BoneFlags {
    Disabled = 0,
    Skinning = 4,
    ParentBone = 8,
}

/// The data for a single mesh draw call.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(strings_offset: u32))]
pub struct Mesh {
    pub vertex_indices_offset: u32,
    pub vertex_buffer0_offset: u32,
    pub vertex_buffer1_offset: u32,

    pub vertex_count: u16,
    pub vertex_flags: VertexFlags,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: strings_offset })]
    #[xc3(offset(u32))]
    pub material1: Option<Material>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: strings_offset })]
    #[xc3(offset(u32))]
    pub material2: Option<Material>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: strings_offset })]
    #[xc3(offset(u32))]
    pub material3: Option<Material>,

    #[br(parse_with = parse_opt_ptr32)]
    #[br(args { inner: strings_offset })]
    #[xc3(offset(u32))]
    pub material4: Option<Material>,

    pub vertex_index_count: u16,
    pub vertex_index_flags: VertexIndexFlags,

    // TODO: padding?
    pub unk: [u32; 3],
}

#[bitsize(16)]
#[derive(DebugBits, TryFromBits, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[br(try_map = |x: u16| x.try_into().map_err(|e| format!("{e:?}")))]
#[bw(map = |&x| u16::from(x))]
pub struct VertexFlags {
    pub uvs: UvType,
    pub colors: ColorType,
    pub uv_count: u4,
    pub normals: NormalType,
    pub bones: BoneType,
}

#[bitsize(16)]
#[derive(DebugBits, TryFromBits, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[br(try_map = |x: u16| x.try_into().map_err(|e| format!("{e:?}")))]
#[bw(map = |&x| u16::from(x))]
pub struct VertexIndexFlags {
    pub unk1: bool, // false
    pub unk2: bool, // false
    /// `true` when not using [BoneType::None].
    pub has_bone_indices_weights: bool,
    pub unk4: u11, // 0
    pub is_triangle_list: bool,
    pub unk5: bool, // false
}

#[bitsize(4)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, TryFromBits, PartialEq, Eq, Clone, Copy)]
pub enum NormalType {
    None = 0,
    NormalsFloat32 = 1,
    NormalsTangentBitangentFloat32 = 3,
    NormalsFloat16 = 6,
    NormalsTangentBitangentFloat16 = 7,
}

#[bitsize(4)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, TryFromBits, PartialEq, Eq, Clone, Copy)]
pub enum BoneType {
    None = 0,
    Float32 = 1,
    Float16 = 2,
    Byte = 4,
}

#[bitsize(3)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, TryFromBits, PartialEq, Eq, Clone, Copy)]
pub enum ColorType {
    None = 0,
    Byte = 1,
    Float16 = 2,
}

#[bitsize(1)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, FromBits, PartialEq, Eq, Clone, Copy)]
pub enum UvType {
    Float16 = 0,
    Float32 = 1, // TODO: wangan midnight?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BoundingSphere {
    pub center: [f32; 3],
    pub radius: f32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(strings_offset: u32))]
pub struct Material {
    pub shader_id: u32,
    pub unk1: u32,
    pub src_factor: SrcFactor,
    pub tex_count: u16,
    pub dst_factor: DstFactor,
    /// The function used to determine what fragment alpha values pass the alpha test.
    pub alpha_func: AlphaFunc,
    /// The reference value for alpha testing calculated as `alpha_test_ref / 255.0`.
    pub alpha_test_ref: u16,
    pub cull_mode: CullMode,
    pub unk2: u32,
    pub unk3: u32,
    pub z_buffer_offset: i32,

    #[br(count = tex_count)]
    pub textures: Vec<MaterialTexture>,

    // TODO: is this the correct way to read all properties?
    #[br(parse_with = until(|prop: &MaterialProperty| prop.size == 0))]
    #[br(args_raw(strings_offset))]
    pub properties: Vec<MaterialProperty>,
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum SrcFactor {
    // TODO: Validate this section.
    One = 0,
    SourceAlpha = 1,
    One2 = 2,
    SourceAlpha2 = 3,
    Zero = 4,
    SourceAlpha3 = 5,
    DestinationAlpha = 6,
    DestinationAlpha7 = 7,
    DestinationColor = 8,
    SrcAlpha3 = 11,
    SrcAlpha4 = 15,
    // TODO: Test these
    Unk16 = 16,
    Unk33 = 33,
    SrcAlpha5 = 37,
}

// TODO: retest these with renderdoc.
// TODO: dst factor + blend op?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum DstFactor {
    Zero = 0,
    OneMinusSourceAlpha = 1,
    One = 2,
    OneReverseSubtract = 3,
    SourceAlpha = 4,
    SourceAlphaReverseSubtract = 5,
    OneMinusDestinationAlpha = 6,
    One2 = 7,
    Zero2 = 8,
    Unk10 = 10,
    OneMinusSourceAlpha2 = 11,
    One3 = 12,  // TODO: Sets src to one?
    Zero5 = 64, // TODO: sets src to one?
    Zero3 = 112,
    One4 = 114,
    OneMinusSourceAlpha3 = 129, // TODO: also affects alpha?
    One5 = 130,                 // TODO: also affects alpha?
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq, Hash)]
#[brw(repr(u16))]
pub enum AlphaFunc {
    Disabled = 0x0,
    Never = 0x200,
    Less = 0x201,
    Equal = 0x202,
    Greater = 0x204,
    NotEqual = 0x205,
    GreaterEqual = 0x206,
    Always = 0x207,
    // TODO: Direct3D for pokken?
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum CullMode {
    Disabled = 0x0,
    Outside = 0x404,
    Inside = 0x405,
    // TODO: pokken?
    Disabled2 = 1,
    Inside2 = 2,
    Outside2 = 3,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MaterialTexture {
    pub hash: u32, // TODO: matches nut gidx hash?
    pub unk1: [u16; 3],
    pub map_mode: MapMode,
    pub wrap_mode_s: WrapMode,
    pub wrap_mode_t: WrapMode,
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    pub mip_detail: MipDetail,
    pub unk2: u8,
    pub unk3: u32,
    pub unk4: u16, // TODO: 7680 for some textures?
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u16))]
pub enum MapMode {
    TexCoord = 0x00,
    EnvCamera = 0x1d00,
    Projection = 0x1e00,
    EnvLight = 0x1ecd,
    EnvSpec = 0x1f00,
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u8))]
pub enum MinFilter {
    LinearMipmapLinear = 0,
    Nearest = 1,
    Linear = 2,
    NearestMipmapLinear = 3,
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u8))]
pub enum MagFilter {
    Unk0 = 0,
    Nearest = 1,
    Linear = 2,
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u8))]
pub enum MipDetail {
    OneMipLevelAnisotropicOff = 0,
    Unk1 = 1,
    OneMipLevelAnisotropicOff2 = 2,
    FourMipLevels = 3,
    FourMipLevelsAnisotropic = 4,
    FourMipLevelsTrilinear = 5,
    FourMipLevelsTrilinearAnisotropic = 6,
}

// TODO: retest these with renderdoc.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, Clone, Copy, PartialEq, Eq)]
#[brw(repr(u8))]
pub enum WrapMode {
    Repeat = 1,
    MirroredRepeat = 2,
    ClampToEdge = 3,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(import_raw(strings_offset: u32))]
pub struct MaterialProperty {
    pub size: u32, // TODO:  size in bytes?
    #[br(parse_with = parse_string_ptr32, offset = strings_offset as u64)]
    #[xc3(offset(u32), align(16))]
    pub name: String,
    pub unk1: [u8; 3],
    pub value_count: u8, // TODO: these aren't all used in practice?
    pub unk2: u32,
    // TODO: Are these always floats?
    #[br(count = value_count)]
    #[br(pad_size_to = size.saturating_sub(16))]
    pub values: Vec<f32>,
}

xc3_write_binwrite_impl!(
    SrcFactor,
    DstFactor,
    AlphaFunc,
    MapMode,
    MinFilter,
    MagFilter,
    MipDetail,
    VertexFlags,
    VertexIndexFlags,
    CullMode,
    WrapMode,
    BoneFlags
);

impl Xc3WriteOffsets for NudOffsets<'_> {
    type Args = ();

    fn write_offsets<W: Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // The names are stored in a single section.
        let string_section = Rc::new(RefCell::new(StringSection::default()));

        // TODO: Find a nicer way to defer material writing.
        let mut meshes = Vec::new();
        for g in &self.mesh_groups.0 {
            string_section.borrow_mut().insert_offset32(&g.name);
            let group_meshes = g.meshes.write(writer, base_offset, data_ptr, endian)?.0;
            meshes.extend(group_meshes);
        }
        for mesh in meshes {
            mesh.write_offsets(
                writer,
                base_offset,
                data_ptr,
                endian,
                string_section.clone(),
            )?;
        }

        let position = writer.stream_position()?;
        align(writer, position, 16, 0u8)?;

        // Remove the header size.
        let index_offset = writer.stream_position()? - 48;

        self.index_buffer.data.xc3_write(writer, endian)?;
        self.vertex_buffer0.data.xc3_write(writer, endian)?;
        self.vertex_buffer1.data.xc3_write(writer, endian)?;
        *data_ptr = (*data_ptr).max(writer.stream_position()?);

        string_section.borrow().write(
            writer,
            *data_ptr,
            data_ptr,
            &WriteOptions {
                start_alignment: 16,
                start_padding_byte: 0,
                string_alignment: 16,
                string_padding_byte: 0,
            },
            endian,
        )?;

        let file_size = *data_ptr;

        // TODO: set_offset should return to the original position after writing.
        self.indices_offset
            .set_offset(writer, index_offset, endian)?;

        self.file_size.set_offset(writer, file_size, endian)?;

        Ok(())
    }
}

impl Xc3WriteOffsets for MeshGroupOffsets<'_> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset32(&self.name);
        self.meshes
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}

impl Xc3WriteOffsets for MeshOffsets<'_> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.material1
            .write_full(writer, base_offset, data_ptr, endian, args.clone())?;
        self.material2
            .write_full(writer, base_offset, data_ptr, endian, args.clone())?;
        self.material3
            .write_full(writer, base_offset, data_ptr, endian, args.clone())?;
        self.material4
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}

impl Xc3WriteOffsets for MaterialOffsets<'_> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.properties
            .write_offsets(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}

impl Xc3WriteOffsets for MaterialPropertyOffsets<'_> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset32(&self.name);
        Ok(())
    }
}

fn align<W: Write>(writer: &mut W, size: u64, align: u64, pad: u8) -> Result<(), std::io::Error> {
    let aligned_size = size.next_multiple_of(align);
    let padding = aligned_size - size;
    writer.write_all(&vec![pad; padding as usize])?;
    Ok(())
}

// Use an impl that doesn't panic on invalid input.
arbitrary_bilge_impl!(u16, VertexFlags, VertexIndexFlags);
