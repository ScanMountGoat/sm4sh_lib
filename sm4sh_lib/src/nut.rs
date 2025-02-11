use std::{io::Cursor, path::Path};

use binrw::{args, binread, BinRead, BinReaderExt, BinResult, FilePtr32};

#[derive(Debug, BinRead)]
pub enum Nut {
    Ntwu(Ntwu),
    Ntp3(Ntp3),
}

// TODO: Are these the same type just with different endianness?
#[derive(Debug, BinRead)]
#[br(magic(b"NTP3"))]
pub struct Ntp3 {
    pub unk1: u16,
    // TODO: more fields
}

#[derive(Debug, BinRead)]
#[br(magic(b"NTWU"))]
pub struct Ntwu {
    pub unk1: u16,
    pub count: u16,
    pub unk2: u64,

    #[br(count = count as usize)]
    pub textures: Vec<Texture>,
}

// TODO: Is caps2 like dds?
#[binread]
#[derive(Debug)]
#[br(stream = r)]
pub struct Texture {
    #[br(try_calc = r.stream_position())]
    base_offset: u64,

    pub size: u32,
    pub unk1: u32,
    pub data_size: u32,
    pub header_size: u16,
    pub unk2: u16,
    pub unk3: u8,
    pub mipmap_count: u8,
    pub unk4: u8,
    pub format: NutFormat,
    pub width: u16,
    pub height: u16,
    pub unk5: u32,
    pub caps2: u32,

    // TODO: all mipmaps?
    #[br(restore_position)]
    data_offset: u32,

    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: args! { count: data_size as usize}})]
    pub data: Vec<u8>,

    pub mipmap_data_offset: u32,

    #[br(parse_with = FilePtr32::parse, offset = base_offset)]
    pub gtx_header: GtxHeader,

    pub unk6: u32,

    // TODO: cube map stuff?
    #[br(count = (header_size - 64) / 4)]
    pub unks: Vec<u32>,

    pub gidx: Gidx,
}

#[derive(Debug, BinRead)]
#[br(repr(u8))]
pub enum NutFormat {
    Bc1 = 0,
    Bc2 = 1,
    Bc3 = 2,
    Rg16 = 8,
    Rgba16 = 12,
    Rgba8 = 14,
    Bgra8 = 16,
    // Rgba8 = 17,
    Unk22 = 22,
}

#[derive(Debug, BinRead)]
#[br(magic(b"GIDX"))]
pub struct Gidx {
    pub unk1: u32,
    pub unk2: (u16, u16),
    pub unk3: u32,
}

#[derive(Debug, BinRead)]
pub struct GtxHeader {
    pub dim: u32,
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub mipmap_count: u32,
    pub format: SurfaceFormat,
    pub aa: u32,
    pub usage: u32,
    pub image_data_size: u32,
    pub image_data_offset: u32,
    pub mipmap_data_size: u32,
    pub mipmap_data_offset: u32,
    pub tile_mode: TileMode,
    pub swizzle: u32,
    pub alignment: u32,
    pub pitch: u32,
    pub mipmap_offsets: [u32; 13],
}

// TODO: Just use the wiiu_swizzle gx2 values directly?
#[derive(Debug, BinRead, Clone, Copy)]
#[brw(repr(u32))]
pub enum SurfaceFormat {
    R5G5B5A1Unorm = 10,
    R8G8B8A8Unorm = 26,
    BC1Unorm = 49,
    BC2Unorm = 50,
    BC3Unorm = 51,
    BC4Unorm = 52,
    BC5Unorm = 53,
}

// TODO: Just use the wiiu_swizzle gx2 values directly?
#[derive(Debug, BinRead, Clone, Copy)]
#[brw(repr(u32))]
pub enum TileMode {
    D1TiledThin1 = 2,
    D2TiledThin1 = 4,
    D2TiledThick = 7,
}

impl Nut {
    pub fn from_file<P: AsRef<Path>>(path: P) -> BinResult<Self> {
        let mut reader = Cursor::new(std::fs::read(path)?);
        reader.read_be()
    }
}

impl SurfaceFormat {
    pub fn block_dim(&self) -> (u32, u32) {
        match self {
            SurfaceFormat::R5G5B5A1Unorm => (1, 1),
            SurfaceFormat::R8G8B8A8Unorm => (1, 1),
            SurfaceFormat::BC1Unorm => (4, 4),
            SurfaceFormat::BC2Unorm => (4, 4),
            SurfaceFormat::BC3Unorm => (4, 4),
            SurfaceFormat::BC4Unorm => (4, 4),
            SurfaceFormat::BC5Unorm => (4, 4),
        }
    }

    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            SurfaceFormat::R5G5B5A1Unorm => 2,
            SurfaceFormat::R8G8B8A8Unorm => 4,
            SurfaceFormat::BC1Unorm => 8,
            SurfaceFormat::BC2Unorm => 16,
            SurfaceFormat::BC3Unorm => 16,
            SurfaceFormat::BC4Unorm => 8,
            SurfaceFormat::BC5Unorm => 16,
        }
    }
}

// 64x64 6 layers BC1 2 mipmaps
// layer 0 mip 0:  448 (0x1c0)
// layer 1 mip 0: 1088 (0x440)
// layer 5 mip 0: 5184 (0x1440)
// layer 0 mip 1: 0, 6144 (0x1800)
// layer 1 mip 1: 128, 6272 (0x1880)
// layer 5 mip 1: 640, 6784 (0x1A80)
impl Texture {
    pub fn deswizzle(&self) -> Result<Vec<u8>, wiiu_swizzle::SwizzleError> {
        // TODO: Avoid unwrap.
        wiiu_swizzle::Gx2Surface {
            dim: wiiu_swizzle::SurfaceDim::from_repr(self.gtx_header.dim).unwrap(),
            width: self.gtx_header.width,
            height: self.gtx_header.height,
            depth_or_array_layers: self.gtx_header.depth_or_array_layers,
            mipmap_count: self.gtx_header.mipmap_count,
            format: wiiu_swizzle::SurfaceFormat::from_repr(self.gtx_header.format as u32).unwrap(),
            aa: wiiu_swizzle::AaMode::from_repr(self.gtx_header.aa).unwrap(),
            usage: self.gtx_header.usage,
            image_data: &self.data[..self.gtx_header.image_data_size as usize],
            mipmap_data: &self.data[self.gtx_header.mipmap_offsets[0] as usize
                ..self.gtx_header.mipmap_offsets[0] as usize
                    + self.gtx_header.mipmap_data_size as usize],
            tile_mode: wiiu_swizzle::TileMode::from_repr(self.gtx_header.tile_mode as u32).unwrap(),
            swizzle: self.gtx_header.swizzle,
            alignment: self.gtx_header.alignment,
            pitch: self.gtx_header.pitch,
            mipmap_offsets: self.gtx_header.mipmap_offsets,
        }
        .deswizzle()
    }
}
