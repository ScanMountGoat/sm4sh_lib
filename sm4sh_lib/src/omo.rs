use bilge::prelude::*;
use binrw::{args, binrw, BinRead, BinWrite, FilePtr32};

// TODO: Better type and variable names.
#[binrw]
#[derive(Debug)]
#[brw(magic(b"OMO "))]
pub struct Omo {
    pub version: (u16, u16),
    pub flags: u32,
    pub unk1: u16,
    pub node_count: u16,
    pub frame_count: u16,
    pub frame_size: u16,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: node_count as usize }})]
    pub nodes: Vec<OmoNode>,

    #[br(temp, restore_position)]
    #[bw(ignore)]
    offsets: [u32; 2],

    // TODO: data for nodes?
    // TODO: Does this always go until the start of keys?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: (offsets[1] - offsets[0]) as usize }})]
    pub inter_data: Vec<u8>,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: frame_count as usize, inner: frame_size } })]
    pub frames: Vec<Frame>,
}

#[derive(Debug, BinRead, BinWrite)]
#[br(import_raw(frame_size_bytes: u16))]
pub struct Frame {
    /// Coefficients for linear interpolation for [OmoNode] values.
    #[br(count = frame_size_bytes / 2)]
    pub keys: Vec<u16>,
}

#[derive(Debug, BinRead, BinWrite)]
pub struct OmoNode {
    pub flags: OmoFlags,
    pub hash: u32,
    pub inter_offset: u32,
    pub key_offset: u32,
}

// TODO: what does 0x1 do?
#[bitsize(32)]
#[derive(DebugBits, TryFromBits, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[br(try_map = |x: u32| x.try_into().map_err(|e| format!("{e:?}")))]
#[bw(map = |&x| u32::from(x))]
pub struct OmoFlags {
    pub unk1: u4,
    pub scale_type: ScaleType,
    pub rotation_type: RotationType,
    pub position_type: PositionType,
    pub position: bool,
    pub rotation: bool,
    pub scale: bool,
    pub unk4: u5,
}

// TODO: interpolate -> Linear?
#[bitsize(8)]
#[derive(TryFromBits, Debug, PartialEq)]
pub enum ScaleType {
    Constant = 0x20,
    Constant2 = 0x30,
    Interpolate = 0x08,
}

#[bitsize(4)]
#[derive(TryFromBits, Debug, PartialEq)]
pub enum RotationType {
    Interpolate = 0x5,
    FConst = 0x6,
    Constant = 0x7,
    Frame = 0xA, // TODO: "keys"?
}

#[bitsize(8)]
#[derive(TryFromBits, Debug, PartialEq)]
pub enum PositionType {
    Frame = 0x04, // TODO: "keys"?
    Interpolate = 0x08,
    Constant = 0x20,
}
