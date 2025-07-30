use binrw::BinRead;

// TODO: binwrite + xc3write?
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(magic(b"NSP3"))]
pub struct Nsh {
    pub file_size: u32,
    pub unk1: u16,
    pub program_count: u16, // gfx2 count / 2?
    pub unk2: [u32; 5],
    // TODO: 0x92000161 is a material hash?
    pub unk: [u32; 56],
}

// TODO: BLK{ to }BLK
