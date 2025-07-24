use binrw::{args, binread, BinRead, FilePtr32};

use crate::parse_string_ptr32;

#[binread]
#[derive(Debug)]
#[br(magic(b"MTA"))]
pub struct Mta {
    pub version: u8, // TODO: difference between MTA3 and MTA4?
    pub unk1: u32,
    pub frame_count: u32,
    pub start_frame: u32,
    pub end_frame: u32,
    pub frame_rate: u32,

    pub mat_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: mat_count as usize }})]
    pub mats: Vec<MatEntryOffset>,

    pub vis_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: vis_count as usize }})]
    pub vis_offset: Vec<VisEntryOffset>,
}

#[derive(Debug, BinRead)]
pub struct MatEntryOffset {
    #[br(parse_with = FilePtr32::parse)]
    pub entry: MatEntry,
}

#[derive(Debug, BinRead)]
pub struct MatEntry {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub mat_hash: u32,

    pub property_count: u32,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: property_count as usize }})]
    pub property_pos: Vec<MatDataOffset>,

    pub has_pat: u8, // TODO: flags?
    pub unk1: u8,
    pub unk2: u8,
    pub unk3: u8,
    pub pat_offset: u32,

    // TODO: Different for mta3 vs mta4?
    // TODO: Optional?
    // #[br(parse_with = parse_string_opt_ptr32)]
    pub second_name_offset: u32,
    pub mat_hash2: u32,
}

#[derive(Debug, BinRead)]
pub struct MatDataOffset {
    #[br(parse_with = FilePtr32::parse)]
    pub data: MatData,
}

#[derive(Debug, BinRead)]
pub struct MatData {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub unk1: u32,
    pub value_count: u32,
    pub frame_count: u32,
    pub unk2: u16,
    pub anim_type: u16,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { inner: value_count, count: frame_count as usize }})]
    pub data: Vec<MatDataValue>,
}

#[derive(Debug, BinRead)]
#[br(import_raw(value_count: u32))]
pub struct MatDataValue {
    #[br(count = value_count)]
    pub values: Vec<f32>,
}

#[derive(Debug, BinRead)]
pub struct VisEntryOffset {
    #[br(parse_with = FilePtr32::parse)]
    pub entry: VisEntry,
}

#[derive(Debug, BinRead)]
pub struct VisEntry {
    #[br(parse_with = parse_string_ptr32)]
    pub name: String,
    pub unk1: u32,
    #[br(parse_with = FilePtr32::parse)]
    pub data: VisEntryData,
}

#[derive(Debug, BinRead)]
pub struct VisEntryData {
    pub frame_count: u32,
    pub unk1: u16,

    pub key_frame_count: u16,
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: key_frame_count as usize }})]
    pub keyframes: Vec<KeyFrame>,
}

#[derive(Debug, BinRead)]
pub struct KeyFrame {
    pub frame_num: u16,
    pub state: u8,
    pub unk1: u8,
}
