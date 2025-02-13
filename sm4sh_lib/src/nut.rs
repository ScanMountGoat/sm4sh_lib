use binrw::{args, binread, BinRead, FilePtr32};

use crate::parse_opt_ptr32;

// TODO: Write support.
#[derive(Debug, BinRead)]
pub enum Nut {
    Ntwu(Ntwu),
    Ntp3(Ntp3),
}

// TODO: Identical to ntwu other than magic?
#[derive(Debug, BinRead)]
#[br(magic(b"NTP3"))]
pub struct Ntp3 {
    pub unk1: u16,
    pub count: u16,
    pub unk2: u64,

    #[br(count = count as usize)]
    pub textures: Vec<Texture>,
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
    #[br(temp, try_calc = r.stream_position())]
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
    pub unk5: u32, // TODO: 0 for ntp3?
    pub caps2: u32,

    // TODO: all mipmaps?
    #[br(parse_with = FilePtr32::parse)]
    #[br(args { offset: base_offset, inner: args! { count: data_size as usize}})]
    pub data: Vec<u8>,

    pub mipmap_data_offset: u32,

    // TODO: null for ntp3 nuts?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    pub gtx_header: Option<GtxHeader>,

    pub unk6: u32,

    // TODO: cube map stuff?
    #[br(count = (header_size - 80) / 4)]
    pub unks: Vec<u32>,

    pub ext: Ext,
    pub gidx: Gidx,
}

// TODO: Test these in game with renderdoc.
#[derive(Debug, BinRead)]
#[br(repr(u8))]
pub enum NutFormat {
    Bc1 = 0,
    Bc2 = 1,
    Bc3 = 2,
    Unk6 = 6,
    Rg16 = 8,
    Rgba16 = 12,
    Rgba8 = 14,
    Bgra8 = 16,
    Rgba82 = 17,
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
#[br(magic(b"eXt\x00"))]
pub struct Ext {
    pub unk1: u32,
    pub unk2: u32,
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

impl Texture {
    pub fn deswizzle(&self) -> Result<Vec<u8>, wiiu_swizzle::SwizzleError> {
        // TODO: Avoid unwrap.
        if let Some(gtx_header) = &self.gtx_header {
            wiiu_swizzle::Gx2Surface {
                dim: wiiu_swizzle::SurfaceDim::from_repr(gtx_header.dim).unwrap(),
                width: gtx_header.width,
                height: gtx_header.height,
                depth_or_array_layers: gtx_header.depth_or_array_layers,
                mipmap_count: gtx_header.mipmap_count,
                format: wiiu_swizzle::SurfaceFormat::from_repr(gtx_header.format as u32).unwrap(),
                aa: wiiu_swizzle::AaMode::from_repr(gtx_header.aa).unwrap(),
                usage: gtx_header.usage,
                image_data: &self.data[..gtx_header.image_data_size as usize],
                mipmap_data: &self.data[gtx_header.mipmap_offsets[0] as usize
                    ..gtx_header.mipmap_offsets[0] as usize + gtx_header.mipmap_data_size as usize],
                tile_mode: wiiu_swizzle::TileMode::from_repr(gtx_header.tile_mode as u32).unwrap(),
                swizzle: gtx_header.swizzle,
                alignment: gtx_header.alignment,
                pitch: gtx_header.pitch,
                mipmap_offsets: gtx_header.mipmap_offsets,
            }
            .deswizzle()
        } else {
            // TODO: How to handle textures without gx2 data?
            Ok(self.data.clone())
        }
    }
}
