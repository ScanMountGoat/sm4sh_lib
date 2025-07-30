use binrw::{args, binread, BinRead, FilePtr32};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{parse_string_opt_ptr32, parse_string_ptr32};

// TODO: different types for MTA2, MTA3, and MTA4
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub enum Mta {
    Mta4(Mta4),
}

#[binread]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(magic(b"MTA4"))]
#[xc3(magic(b"MTA4"))]
pub struct Mta4 {
    pub unk1: u32,
    pub frame_count: u32,
    pub start_frame: u32,
    pub end_frame: u32,
    pub frame_rate: u32,

    pub mat_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: mat_count as usize }})]
    #[xc3(offset(u32))]
    pub material_entries: Vec<MatEntryOffset>,

    pub vis_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: vis_count as usize }})]
    #[xc3(offset(u32))]
    pub visibility_entries: Vec<VisEntryOffset>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MatEntryOffset {
    #[br(parse_with = FilePtr32::parse)]
    #[xc3(offset(u32), align(32))]
    pub entry: MatEntry,
}

#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct MatEntry {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub name: String,

    pub mat_hash: u32,

    pub property_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: property_count as usize }})]
    #[xc3(offset(u32))]
    pub properties: Vec<MatDataOffset>,

    pub pattern_count: u8,
    pub unk1: u8,
    pub unk2: u8,
    pub unk3: u8,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: pattern_count as usize } })]
    #[xc3(offset(u32))]
    pub pattern_entries: Vec<PatternEntryOffset>,

    // TODO: not present for v3
    #[br(parse_with = parse_string_opt_ptr32)]
    #[xc3(offset(u32), align(4))]
    pub second_name_offset: Option<String>,
    pub mat_hash2: u32,
}

// TODO: make this generic since the alignment is always 32
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MatDataOffset {
    #[br(parse_with = FilePtr32::parse)]
    #[xc3(offset(u32), align(32))]
    pub data: MatData,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MatData {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32), align(16))]
    pub name: String,

    pub unk1: u32,
    pub value_count: u32,
    pub frame_count: u32,
    pub unk2: u16,
    pub anim_type: u16,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { inner: value_count, count: frame_count as usize }})]
    #[xc3(offset(u32), align(32))]
    pub data: Vec<MatDataValue>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(value_count: u32))]
pub struct MatDataValue {
    #[br(count = value_count)]
    pub values: Vec<f32>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VisEntryOffset {
    #[br(parse_with = FilePtr32::parse)]
    #[xc3(offset(u32), align(32))]
    pub entry: VisEntry,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VisEntry {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32), align(32))]
    pub name: String,

    pub unk1: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[xc3(offset(u32), align(32))]
    pub data: VisEntryData,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VisEntryData {
    pub frame_count: u32,
    pub unk1: u16,

    pub key_frame_count: u16,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: key_frame_count as usize }})]
    #[xc3(offset(u32), align(32))]
    pub keyframes: Vec<VisKeyFrame>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct VisKeyFrame {
    pub frame_num: u16,
    pub state: u8,
    pub unk1: u8,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PatternEntryOffset {
    #[br(parse_with = FilePtr32::parse)]
    #[xc3(offset(u32), align(32))]
    pub entry: PatternEntry,
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
pub struct PatternEntry {
    pub default_tex_id: u32,

    pub key_frame_count: u32,

    #[br(temp, restore_position)]
    key_frames_offset: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: key_frame_count as usize }})]
    #[xc3(offset(u32))]
    pub key_frames: Vec<PatternKeyFrame>,

    pub frame_count: u32,

    // TODO: variable padding?
    #[br(temp, try_calc = r.stream_position())]
    end_offset: u64,

    #[br(count = key_frames_offset as usize - end_offset as usize)]
    pub unks: Vec<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct PatternKeyFrame {
    pub tex_id: u32,
    pub frame_num: u32,
}

impl Xc3WriteOffsets for Mta4Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        if !self.material_entries.data.is_empty() {
            self.material_entries
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        if !self.visibility_entries.data.is_empty() {
            self.visibility_entries
                .write_full(writer, base_offset, data_ptr, endian, ())?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for MatEntryOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        self.name
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.second_name_offset
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.properties
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.pattern_entries
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}
