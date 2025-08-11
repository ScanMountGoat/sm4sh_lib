use binrw::{helpers::until_eof, BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"NSP3"))]
pub struct Nsh {
    pub file_size: u32,
    pub unk1: u16,
    pub program_count: u16, // gfx2 count / 2?
    pub unk2: [u32; 5],     // TODO: all 0?
    pub unk3: u32,          // TODO: 0x92000161 is a material hash?
    pub unk4: [u32; 5],     // TODO: [0x02, 0x60, 0x70, 0x80, 0x90]
    pub unk5: u32,          // TODO: depends on file size?
    pub unk6: [u32; 17],    // TODO: all 0?
    pub unk7: [(u32, u32); 16],

    // TODO: GFX2 shaders with BLK{ to }BLK
    #[br(parse_with = until_eof)]
    pub data: Vec<u8>,
}
