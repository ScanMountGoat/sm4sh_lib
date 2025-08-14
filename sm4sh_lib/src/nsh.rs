use binrw::{helpers::until, BinRead, BinWrite};

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"NSP3"))]
pub struct Nsh {
    pub file_size: u32,
    pub unk1: u16,
    pub program_count: u16,
    pub unk2: [u32; 5],  // TODO: all 0?
    pub unk3: u32,       // TODO: 0x92000161 is a material shader ID?
    pub unk4: [u32; 5],  // TODO: [0x02, 0x60, 0x70, 0x80, 0x90]
    pub unk5: u32,       // TODO: depends on file size?
    pub unk6: [u32; 17], // TODO: all 0?
    pub unk7: [(u32, u32); 16],

    #[br(count = program_count * 2)]
    #[brw(align_after = 128)]
    pub shaders: Vec<Gfx2>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"Gfx2"))]
pub struct Gfx2 {
    pub header_size: u32,    // 32
    pub major_version: u32,  // 7
    pub minor_version: u32,  // 1
    pub gpu_version: u32,    // 2,
    pub alignment_mode: u32, // TODO: 0 or 1
    // TODO: padding
    pub unk: [u32; 2],

    // TODO: Extra data after eof block?
    #[br(parse_with = until(|b: &Block| b.block_type == BlockType::EndOfFile))]
    #[brw(align_after = 128)]
    pub blocks: Vec<Block>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"BLK{"))]
pub struct Block {
    pub header_size: u32,   // 32
    pub major_version: u32, // 1
    pub minor_version: u32, // 0
    #[br(dbg)]
    pub block_type: BlockType,
    #[br(dbg)]
    pub data_size: u32,
    // TODO: padding
    pub unk: [u32; 2],

    #[br(count = data_size)]
    pub data: Vec<u8>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u32))]
pub enum BlockType {
    EndOfFile = 1,
    Padding = 2,
    VertexShaderHeader = 3,
    VertexShaderProgram = 5,
    PixelShaderHeader = 6,
    PixelShaderProgram = 7,
    GeometryShaderHeader = 8,
    GeometryShaderProgram = 9,
    GeometryShaderCopyProgram = 10,
    TextureHeader = 11,
    TextureImageData = 12,
    TextureMipmapData = 13,
}
