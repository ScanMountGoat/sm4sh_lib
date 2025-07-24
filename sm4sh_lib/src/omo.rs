use binrw::{args, binrw, BinRead, BinWrite, FilePtr32};
use bitflags::bitflags;

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
    pub inter_offset: Vec<u8>,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { inner: args! { count: frame_size as usize }})]
    pub keys: Vec<u8>, // TODO: u16?
}

#[derive(Debug, BinRead, BinWrite)]
pub struct OmoNode {
    #[br(map(|x: u32| OmoFlags::from_bits_retain(x)))]
    #[bw(map(|x| x.bits()))]
    pub flags: OmoFlags,
    pub hash: u32,
    pub inter_offset: u32,
    pub key_offset: u32,
}

// TODO: Are some of these meant to be mutually exclusive?
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OmoFlags: u32 {
        const POSITION = 0x01000000;
        const ROTATION = 0x02000000;
        const SCALE = 0x04000000;
        const POSITION_INTER = 0x00080000;
        const POSITION_CONST = 0x00200000;
        const ROTATION_INTER = 0x00005000;
        const ROTATION_FCONST = 0x00006000;
        const ROTATION_CONST = 0x00007000;
        const ROTATION_FRAME = 0x0000A000;
        const SCALE_CONST = 0x00000200;
        const SCALE_CONST2 = 0x00000300;
        const SCALE_INTER = 0x00000080;
    }
}
