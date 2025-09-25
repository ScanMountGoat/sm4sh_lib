use binrw::{BinRead, BinWrite};
use xc3_write::{
    Xc3Write, Xc3WriteOffsets,
    strings::{StringSectionUnique, WriteOptions},
};

use crate::{parse_count32_offset32, parse_string_ptr32, xc3_write_binwrite_impl};

#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Gx2VertexShader {
    pub registers: Gx2VertexShaderRegisters,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_blocks: Vec<UniformBlock>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_vars: Vec<UniformVar>,

    pub unk9: [u32; 4], // TODO: initial values and loop vars

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub sampler_vars: Vec<SamplerVar>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub attributes: Vec<Attribute>,

    pub ring_item_size: u32,
    pub has_stream_out: u32,
    pub stream_out_stride: [u32; 4],
    pub r_buffer: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Gx2VertexShaderRegisters {
    pub sq_pgm_resources_vs: u32,
    pub vgt_primitiveid_en: u32,
    pub spi_vs_out_config: u32,
    pub num_spi_vs_out_id: u32,
    pub spi_vs_out_id: [u32; 10],
    pub pa_cl_vs_out_cntl: u32,
    pub sq_vtx_semantic_clear: u32,
    pub num_sq_vtx_semantic: u32,
    pub sq_vtx_semantic: [u32; 32],
    pub vgt_strmout_buffer_en: u32,
    pub vgt_vertex_reuse_block_cntl: u32,
    pub vgt_hos_reuse_depth: u32,
}

#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Gx2PixelShader {
    pub registers: Gx2PixelShaderRegisters,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32), align(4096))]
    pub program_binary: Vec<u8>,

    pub shader_mode: ShaderMode,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_blocks: Vec<UniformBlock>,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub uniform_vars: Vec<UniformVar>,

    pub unk9: [u32; 4], // TODO: initial values and loop vars

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub sampler_vars: Vec<SamplerVar>,

    pub r_buffer: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Gx2PixelShaderRegisters {
    pub sq_pgm_resources_ps: u32,
    pub sq_pgm_exports_ps: u32,
    pub spi_ps_in_control_0: u32,
    pub spi_ps_in_control_1: u32,
    pub num_spi_ps_input_cntl: u32,
    pub spi_ps_input_cntls: [u32; 32],
    pub cb_shader_mask: u32,
    pub cb_shader_control: u32,
    pub db_shader_control: u32,
    pub spi_input_z: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UniformBlock {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct UniformVar {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub offset: u32,
    /// The index into [uniform_blocks](struct.FragmentShader.html#structfield.uniform_blocks)
    /// or `-1` if this uniform is not part of a buffer.
    pub uniform_block_index: i32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Attribute {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
    pub data_type: VarType,
    pub count: u32,
    pub location: u32,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct SamplerVar {
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
    pub sampler_type: SamplerType,
    pub location: u32,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum ShaderMode {
    UniformRegister = 0, // TODO: uniforms but no buffers?
    UniformBlock = 1,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum VarType {
    Void = 0,
    Bool = 1,
    Float = 4,
    Vec2 = 9,
    Vec3 = 10,
    Vec4 = 11,
    IVec2 = 15,
    IVec4 = 17,
    UVec4 = 20,
    Mat2x4 = 23,
    Mat3x4 = 26,
    Mat4 = 29,
}

#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy, Hash)]
#[brw(repr(u32))]
pub enum SamplerType {
    D1 = 0,
    D2 = 1,
    Unk2 = 2,
    Unk3 = 3,
    Cube = 4,
    Unk10 = 10,
    Unk13 = 13,
}

xc3_write_binwrite_impl!(VarType, ShaderMode, SamplerType);

impl Xc3WriteOffsets for Gx2VertexShaderOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let mut strings = StringSectionUnique::default();
        let blocks = self
            .uniform_blocks
            .write(writer, base_offset, data_ptr, endian)?;
        for b in blocks.0 {
            strings.insert_offset32(&b.name);
        }
        let uniforms = self
            .uniform_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for u in uniforms.0 {
            strings.insert_offset32(&u.name);
        }
        let samplers = self
            .sampler_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for s in samplers.0 {
            strings.insert_offset32(&s.name);
        }
        let attributes = self
            .attributes
            .write(writer, base_offset, data_ptr, endian)?;
        for a in attributes.0 {
            strings.insert_offset32(&a.name);
        }
        strings.write(
            writer,
            base_offset,
            data_ptr,
            &WriteOptions::default(),
            endian,
        )?;
        self.program_binary
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}

impl Xc3WriteOffsets for Gx2PixelShaderOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        // Different order than field order.
        let mut strings = StringSectionUnique::default();
        let blocks = self
            .uniform_blocks
            .write(writer, base_offset, data_ptr, endian)?;
        for b in blocks.0 {
            strings.insert_offset32(&b.name);
        }
        let uniforms = self
            .uniform_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for u in uniforms.0 {
            strings.insert_offset32(&u.name);
        }
        let samplers = self
            .sampler_vars
            .write(writer, base_offset, data_ptr, endian)?;
        for s in samplers.0 {
            strings.insert_offset32(&s.name);
        }
        strings.write(
            writer,
            base_offset,
            data_ptr,
            &WriteOptions::default(),
            endian,
        )?;
        self.program_binary
            .write_full(writer, base_offset, data_ptr, endian, args)?;
        Ok(())
    }
}
