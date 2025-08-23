use std::io::{Cursor, Seek, Write};

use binrw::{helpers::until, BinRead, BinReaderExt, BinResult, BinWrite};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{
    file_read_impl, file_write_full_impl,
    gx2::{Attribute, Gx2PixelShader, Gx2VertexShader, SamplerVar, UniformBlock, UniformVar},
};

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

    #[br(count = program_count)]
    pub programs: Vec<ShaderProgram>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct ShaderProgram {
    pub vertex: Gfx2Shader,
    pub pixel: Gfx2Shader,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
pub struct Gfx2Shader {
    pub gfx2: Gfx2,
    #[br(parse_with = parse_extra_data)]
    pub extra_data: Vec<u8>, // TODO: non empty for every other gfx2?
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

    #[br(parse_with = until(|b: &Block| b.block_type == BlockType::EndOfFile))]
    pub blocks: Vec<Block>,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"BLK{"))]
pub struct Block {
    pub header_size: u32,   // 32
    pub major_version: u32, // 1
    pub minor_version: u32, // 0
    pub block_type: BlockType,
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

#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"}BLK"))]
pub struct RelocationInfo {
    pub size: u32,               // 40
    pub unk1: u32,               // 0
    pub shader_string_size: u32, // shader structs + strings
    pub shader_strings_offset: u32,
    pub strings_size: u32,
    pub strings_offset: u32,
    pub unk2: u32, //0
    pub relocation_count: u32,
    pub relocation_table_offset: u32,
}

fn parse_extra_data<R: std::io::Read + Seek>(
    reader: &mut R,
    endian: binrw::Endian,
    args: (),
) -> BinResult<Vec<u8>> {
    // TODO: Is this count stored anywhere?
    let mut extra_data = Vec::new();
    while let Ok(bytes) = <[u8; 4]>::read_options(reader, endian, args) {
        match &bytes[..] {
            b"Gfx2" => {
                reader.seek(std::io::SeekFrom::Current(-4))?;
                break;
            }
            _ => extra_data.extend(bytes),
        }
    }
    Ok(extra_data)
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub enum Gx2Shader {
    Vertex(Gx2VertexShader),
    Pixel(Gx2PixelShader),
}

impl Gx2Shader {
    pub fn program_binary(&self) -> &[u8] {
        match self {
            Gx2Shader::Vertex(v) => &v.program_binary,
            Gx2Shader::Pixel(p) => &p.program_binary,
        }
    }

    pub fn uniform_blocks(&self) -> &[UniformBlock] {
        match self {
            Gx2Shader::Vertex(v) => &v.uniform_blocks,
            Gx2Shader::Pixel(p) => &p.uniform_blocks,
        }
    }

    pub fn uniform_vars(&self) -> &[UniformVar] {
        match self {
            Gx2Shader::Vertex(v) => &v.uniform_vars,
            Gx2Shader::Pixel(p) => &p.uniform_vars,
        }
    }

    pub fn sampler_vars(&self) -> &[SamplerVar] {
        match self {
            Gx2Shader::Vertex(_) => &[],
            Gx2Shader::Pixel(p) => &p.sampler_vars,
        }
    }

    pub fn attributes(&self) -> &[Attribute] {
        match self {
            Gx2Shader::Vertex(v) => &v.attributes,
            Gx2Shader::Pixel(_) => &[],
        }
    }
}

file_read_impl!(
    binrw::Endian::Big,
    Gx2Shader,
    Gx2VertexShader,
    Gx2PixelShader
);
file_write_full_impl!(xc3_write::Endian::Big, Gx2Shader);

impl Gfx2 {
    // TODO: Create a gx2 struct instead to support saving with different endianness.
    pub fn gx2_be_bytes(&self) -> BinResult<Vec<u8>> {
        let mut writer = Cursor::new(Vec::new());

        let mut binary_pos = 4096;
        for block in &self.blocks {
            if matches!(
                block.block_type,
                BlockType::VertexShaderHeader | BlockType::PixelShaderHeader
            ) {
                let mut block_reader = Cursor::new(&block.data);
                block_reader.seek(std::io::SeekFrom::End(-40))?;
                let rlt: RelocationInfo = block_reader.read_be()?;

                // TODO: Don't assume this starts at 0?
                let mut data = block.data[..rlt.shader_string_size as usize].to_vec();

                // Align program data.
                binary_pos = rlt.shader_string_size.next_multiple_of(4096);

                // Relocate offsets.
                block_reader.set_position((rlt.relocation_table_offset & 0xFFFFF) as u64);
                for _ in 0..rlt.relocation_count {
                    // AAABBBBB with A tag and B offset.
                    // TODO: offset type with 0xCA7... for string and 0xD06... for data
                    let offset: u32 = block_reader.read_be()?;
                    let offset_pos = (offset & 0xFFFFF) as usize;

                    let old_offset =
                        u32::from_be_bytes(data[offset_pos..offset_pos + 4].try_into().unwrap());
                    let new_offset = old_offset & 0xFFFFF;
                    data[offset_pos..offset_pos + 4].copy_from_slice(&new_offset.to_be_bytes());
                }

                // TODO: Why isn't the program offset in the relocation information?
                if block.block_type == BlockType::VertexShaderHeader {
                    data[212..216].copy_from_slice(&binary_pos.to_be_bytes());
                } else {
                    data[168..172].copy_from_slice(&binary_pos.to_be_bytes());
                }

                writer.write_all(&data)?;
            } else if matches!(
                block.block_type,
                BlockType::VertexShaderProgram | BlockType::PixelShaderProgram
            ) {
                writer.set_position(binary_pos as u64);
                writer.write_all(&block.data)?;
            }
        }

        Ok(writer.into_inner())
    }

    pub fn gx2_shader(&self) -> BinResult<Gx2Shader> {
        let bytes = self.gx2_be_bytes()?;
        let mut reader = Cursor::new(bytes);
        reader.read_be()
    }
}
