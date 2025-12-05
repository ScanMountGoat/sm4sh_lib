use binrw::{BinRead, BinWrite};

// TODO: Find a better way to detect endianness.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[br(little)]
#[brw(magic(b" BWS"))]
pub struct Sb {
    pub version: (u16, u16),
    pub count: u32,

    #[br(count = count)]
    pub entries: Vec<SbEntry>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct SbEntry {
    pub hash: u32,
    pub param1_1: f32,
    pub param1_2: u32,
    pub param1_3: u32,
    pub param2_1: f32,
    pub param2_2: f32,
    pub param2_3: u32,
    pub rx1: f32,
    pub rx2: f32,
    pub ry1: f32,
    pub ry2: f32,
    pub rz1: f32,
    pub rz2: f32,
    pub bone_hashes: [u32; 8],
    pub unks1: [f32; 4],
    pub unks2: [f32; 6],
    pub factor: f32,
    pub ints: [u32; 3],
}
