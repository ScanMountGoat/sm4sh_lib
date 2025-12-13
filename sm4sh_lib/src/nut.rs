use binrw::{BinRead, BinWrite, binread};
use bitflags::bitflags;
use image_dds::{Surface, ddsfile::Dds};
use thiserror::Error;
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use crate::{parse_opt_ptr32, parse_ptr32_count, xc3_write_binwrite_impl};

// TODO: Same inner type for all variants?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub enum Nut {
    Ntwu(Ntwu),
    Ntp3(Ntp3),
}

// TODO: Identical to ntwu other than magic?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"NTP3"))]
#[xc3(magic(b"NTP3"))]
pub struct Ntp3 {
    pub unk1: u16,
    pub count: u16,
    pub unk2: u64,

    // TODO: Are these always not tiled?
    #[br(count = count as usize)]
    pub textures: Vec<Texture>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"NTWU"))]
#[xc3(magic(b"NTWU"))]
pub struct Ntwu {
    pub unk1: u16,
    pub count: u16,
    pub unk2: u64,

    #[br(count = count as usize)]
    pub textures: Vec<Texture>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Texture {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub size: u32, // TODO: data size + header size?
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
    pub caps2: Caps2,

    // TODO: NTP3 image data isn't aligned at all?
    // TODO: Separate type for non tiled texture?

    // TODO: all mipmaps?
    // TODO: Some are aligned to 8192?
    #[br(parse_with = parse_ptr32_count(data_size as usize), offset = base_offset)]
    #[xc3(offset(u32), align(4096))]
    pub data: Vec<u8>,

    // TODO: calculate this on export?
    pub mipmap_data_offset: u32,

    // TODO: null for ntp3 nuts?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub gtx_header: Option<GtxHeader>,

    pub unk6: u32,

    // TODO: cube map stuff?
    #[br(count = (header_size - 80) / 4)]
    pub unks: Vec<u32>,

    pub ext: Ext,
    pub gidx: Gidx,
}

// Identical to flags used for DDS.
// https://github.com/SiegeEngine/ddsfile/blob/3126d7694e42f7b6c84a19d550c5b61aeb8b5869/src/header.rs#L364-L391
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Caps2: u32 {
        /// Required for a cube map
        const CUBEMAP = 0x200;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_POSITIVEX = 0x400;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_NEGATIVEX = 0x800;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_POSITIVEY = 0x1000;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_NEGATIVEY = 0x2000;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_POSITIVEZ = 0x4000;
        /// Required when these surfaces are stored in a cubemap
        const CUBEMAP_NEGATIVEZ = 0x8000;
        /// Required for a volume texture
        const VOLUME = 0x200000;
        /// Identical to setting all cubemap direction flags
        const CUBEMAP_ALLFACES = Self::CUBEMAP_POSITIVEX.bits()
            | Self::CUBEMAP_NEGATIVEX.bits()
            | Self::CUBEMAP_POSITIVEY.bits()
            | Self::CUBEMAP_NEGATIVEY.bits()
            | Self::CUBEMAP_POSITIVEZ.bits()
            | Self::CUBEMAP_NEGATIVEZ.bits();
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Caps2 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let value: u32 = u.arbitrary()?;
        Self::from_bits(value).ok_or(arbitrary::Error::IncorrectFormat)
    }
}

impl BinRead for Caps2 {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let pos = reader.stream_position()?;
        let value = u32::read_options(reader, endian, ())?;
        Self::from_bits(value).ok_or(binrw::Error::AssertFail {
            pos,
            message: format!("Invalid CAPS2 {value:X?}"),
        })
    }
}

impl Xc3Write for Caps2 {
    type Offsets<'a>
        = ()
    where
        Self: 'a;

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        self.bits().xc3_write(writer, endian)
    }
}

// TODO: Test these in game with renderdoc.
// TODO: gtx format takes priority if present?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u8))]
pub enum NutFormat {
    BC1Unorm = 0,
    BC2Unorm = 1,
    BC3Unorm = 2,
    Bgr5A1Unorm = 6,  // TODO: are the channels the same as rgb5a1?
    Bgr5A1Unorm2 = 8, // TODO: are the channels the same as rgb5a1?
    B5G6R5Unorm = 10,
    Rgb5A1Unorm = 12,
    Rgba8Unorm = 14,
    R32Float = 16,
    Rgba82 = 17,
    BC5Unorm = 22,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"GIDX"))]
pub struct Gidx {
    pub unk1: u32,
    pub hash: u32, // TODO: does this match with material texture hash?
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"eXt\x00"))]
pub struct Ext {
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct GtxHeader {
    pub dim: SurfaceDim,
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
    pub mipmap_count: u32,
    pub format: SurfaceFormat,
    pub aa: AaMode,
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
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
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u32))]
pub enum TileMode {
    D1TiledThin1 = 2,
    D2TiledThin1 = 4,
    D2TiledThick = 7,
}

// TODO: Just use the wiiu_swizzle gx2 values directly?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u32))]
pub enum AaMode {
    X1 = 0,
    X2 = 1,
    X4 = 2,
    X8 = 3,
}

// TODO: Just use the wiiu_swizzle gx2 values directly?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Eq, Clone, Copy)]
#[brw(repr(u32))]
pub enum SurfaceDim {
    D1 = 0,
    D2 = 1,
    D3 = 2,
    Cube = 3,
}

#[derive(Debug, Error)]
pub enum CreateSurfaceError {
    #[error("error deswizzling surface")]
    SwizzleError(#[from] wiiu_swizzle::SwizzleError),

    #[error("image format {0:?} is not supported")]
    UnsupportedImageFormat(NutFormat),
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
        if let Some(gtx_header) = &self.gtx_header {
            // TODO: Avoid unwrap.
            wiiu_swizzle::Gx2Surface {
                dim: wiiu_swizzle::SurfaceDim::from_repr(gtx_header.dim as u32).unwrap(),
                width: gtx_header.width,
                height: gtx_header.height,
                depth_or_array_layers: gtx_header.depth_or_array_layers,
                mipmap_count: gtx_header.mipmap_count,
                format: wiiu_swizzle::SurfaceFormat::from_repr(gtx_header.format as u32).unwrap(),
                aa: wiiu_swizzle::AaMode::from_repr(gtx_header.aa as u32).unwrap(),
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
            Ok(self.data.clone())
        }
    }

    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, CreateSurfaceError> {
        let mut data = self.deswizzle()?;
        if self.format == NutFormat::Rgb5A1Unorm {
            // image_dds only supports Bgr5A1Unorm.
            swap_red_blue_bgr5a1(&mut data);
        }

        Ok(Surface {
            width: self.width as u32,
            height: self.height as u32,
            depth: 1,
            layers: if self.caps2 == Caps2::CUBEMAP.union(Caps2::CUBEMAP_ALLFACES) {
                6
            } else {
                1
            },
            mipmaps: self.mipmap_count as u32,
            image_format: self.format.try_into()?,
            data,
        })
    }

    pub fn to_dds(&self) -> Result<Dds, image_dds::CreateDdsError> {
        // TODO: Create error type to avoid unwrap.
        self.to_surface().unwrap().to_dds()
    }
}

impl TryFrom<NutFormat> for image_dds::ImageFormat {
    type Error = CreateSurfaceError;

    fn try_from(value: NutFormat) -> Result<Self, Self::Error> {
        match value {
            NutFormat::BC1Unorm => Ok(image_dds::ImageFormat::BC1RgbaUnorm),
            NutFormat::BC2Unorm => Ok(image_dds::ImageFormat::BC2RgbaUnorm),
            NutFormat::BC3Unorm => Ok(image_dds::ImageFormat::BC3RgbaUnorm),
            NutFormat::Bgr5A1Unorm => Ok(image_dds::ImageFormat::Bgr5A1Unorm),
            NutFormat::Bgr5A1Unorm2 => Ok(image_dds::ImageFormat::Bgr5A1Unorm),
            NutFormat::B5G6R5Unorm => Err(CreateSurfaceError::UnsupportedImageFormat(value)),
            NutFormat::Rgb5A1Unorm => Ok(image_dds::ImageFormat::Bgr5A1Unorm),
            NutFormat::Rgba8Unorm => Ok(image_dds::ImageFormat::Rgba8Unorm),
            NutFormat::R32Float => Ok(image_dds::ImageFormat::R32Float),
            NutFormat::Rgba82 => Ok(image_dds::ImageFormat::Rgba8Unorm),
            NutFormat::BC5Unorm => Ok(image_dds::ImageFormat::BC5RgUnorm),
        }
    }
}

impl From<image_dds::ImageFormat> for NutFormat {
    fn from(value: image_dds::ImageFormat) -> Self {
        match value {
            image_dds::ImageFormat::Rgba8Unorm => NutFormat::Rgba8Unorm,
            image_dds::ImageFormat::R32Float => NutFormat::R32Float,
            image_dds::ImageFormat::BC1RgbaUnorm => NutFormat::BC1Unorm,
            image_dds::ImageFormat::BC2RgbaUnorm => NutFormat::BC2Unorm,
            image_dds::ImageFormat::BC3RgbaUnorm => NutFormat::BC3Unorm,
            image_dds::ImageFormat::BC5RgUnorm => NutFormat::BC5Unorm,
            image_dds::ImageFormat::Bgr5A1Unorm => NutFormat::Rgb5A1Unorm,
            _ => todo!(),
        }
    }
}

fn swap_red_blue_bgr5a1(data: &mut [u8]) {
    // TODO: Move this logic to image_dds?
    data.chunks_exact_mut(2).for_each(|c| {
        // Most significant bit -> GGGBBBBBARRRRRGG -> least significant bit.
        let mut bytes = u16::from_be_bytes(c.try_into().unwrap());
        let r = (bytes >> 2) & 0x1F;
        let b = (bytes >> 8) & 0x1F;
        bytes = bytes & 0b1110000010000011 | (r << 8) | (b << 2);
        c.copy_from_slice(&bytes.to_be_bytes());
    });
}

xc3_write_binwrite_impl!(
    NutFormat,
    Ext,
    Gidx,
    SurfaceFormat,
    TileMode,
    AaMode,
    SurfaceDim
);

impl Xc3WriteOffsets for Ntp3Offsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        for t in &self.textures.0 {
            t.data
                .write_full(writer, t.base_offset, data_ptr, endian, args)?;
        }
        for t in &self.textures.0 {
            t.gtx_header
                .write_full(writer, t.base_offset, data_ptr, endian, args)?;
        }
        Ok(())
    }
}

impl Xc3WriteOffsets for NtwuOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        for t in &self.textures.0 {
            t.data
                .write_full(writer, t.base_offset, data_ptr, endian, args)?;
        }
        for t in &self.textures.0 {
            t.gtx_header
                .write_full(writer, t.base_offset, data_ptr, endian, args)?;
        }
        Ok(())
    }
}
