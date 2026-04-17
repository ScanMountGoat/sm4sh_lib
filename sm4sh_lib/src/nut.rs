use std::io::SeekFrom;

use binrw::{BinRead, BinWrite, binread};
use bitflags::bitflags;
use image_dds::{Surface, ddsfile::Dds, mip_dimension};
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

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"NTP3"))]
#[xc3(magic(b"NTP3"))]
pub struct Ntp3 {
    // TODO: 256 has texture data texture data
    // TODO: 512 has texture texture data data
    pub inner: Ntp3Inner,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub enum Ntp3Inner {
    #[br(magic(0x0100u16))]
    #[xc3(magic(0x0100u16))]
    V1(Ntp3InnerV1),

    #[br(magic(0x0200u16))]
    #[xc3(magic(0x0200u16))]
    V2(Ntp3InnerV2),
}

// TODO: make this generic?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Ntp3InnerV1 {
    pub count: u16,
    pub unk2: u64, // 0

    #[br(count = count as usize)]
    pub textures: Vec<Ntp3TextureV1>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Ntp3InnerV2 {
    pub count: u16,
    pub unk2: u64, // 0

    #[br(count = count as usize)]
    pub textures: Vec<Ntp3TextureV2>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
#[br(magic(b"NTWU"))]
#[xc3(magic(b"NTWU"))]
pub struct Ntwu {
    pub version: u16, // 512, 526
    pub count: u16,
    pub unk2: u64, // 0

    #[br(count = count as usize)]
    pub textures: Vec<NtwuTexture>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct NtwuTexture {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub header: TextureHeader,

    // TODO: NTWU is 4096 or 8192 aligned?
    /// Data for all mipmaps.
    #[br(parse_with = parse_ptr32_count(header.data_size as usize), offset = base_offset)]
    #[xc3(offset(u32), align(4096))]
    pub data: Vec<u8>,

    // TODO: calculate this on export?
    pub mipmap_data_offset: u32,

    // TODO: null for ntp3 nuts?
    #[br(parse_with = parse_opt_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub gtx_header: Option<GtxHeader>,

    pub unk6: u32,

    /// The size in bytes for the base image and all mipmaps.
    /// TODO: this is not present if mipmap count is 1?
    // TODO: This is completely different for cubemaps?
    #[br(count = (header.header_size - 80) / 4)]
    pub unk_sizes: Vec<u32>,

    pub ext: Ext,
    pub gidx: Gidx,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Ntp3TextureV1 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub header: TextureHeader,

    // TODO: only this data offset field differs?

    // TODO: calculate this on export?
    pub mipmap_data_offset: u32,

    // TODO: always null for ntp3 nuts?
    #[br(assert(gtx_header == 0))]
    pub gtx_header: u32,

    pub unk6: u32,

    /// The size in bytes for the base image and all mipmaps.
    /// TODO: this is not present if mipmap count is 1?
    // TODO: This is completely different for cubemaps?
    #[br(count = (header.header_size - 76) / 4)]
    pub unk_sizes: Vec<u32>,

    pub ext: Ext,
    pub gidx: Gidx,

    // Don't restore the position since the next texture follows immediately for v1.
    /// Data for all mipmaps.
    #[br(seek_before = SeekFrom::Start(base_offset + header.header_size as u64), count = header.data_size as usize)]
    pub data: Vec<u8>,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(stream = r)]
#[xc3(base_offset)]
pub struct Ntp3TextureV2 {
    #[br(temp, try_calc = r.stream_position())]
    base_offset: u64,

    pub header: TextureHeader,

    // TODO: NTWU is 4096 or 8192 aligned?
    /// Data for all mipmaps.
    #[br(parse_with = parse_ptr32_count(header.data_size as usize), offset = base_offset)]
    #[xc3(offset(u32))]
    pub data: Vec<u8>,

    // TODO: calculate this on export?
    pub mipmap_data_offset: u32,

    // TODO: always null for ntp3 nuts?
    #[br(assert(gtx_header == 0))]
    pub gtx_header: u32,

    pub unk6: u32,

    /// The size in bytes for the base image and all mipmaps.
    /// TODO: this is not present if mipmap count is 1?
    // TODO: This is completely different for cubemaps?
    #[br(count = (header.header_size - 80) / 4)]
    pub unk_sizes: Vec<u32>,

    pub ext: Ext,
    pub gidx: Gidx,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct TextureHeader {
    pub size: u32, // TODO: data size + header size?
    pub unk1: u32,
    pub data_size: u32,
    pub header_size: u16, // TODO: aligned to 16?
    pub unk2: u16,
    pub unk3: u8,
    pub mipmap_count: u8,
    pub unk4: u8,
    pub format: NutFormat,
    pub width: u16,
    pub height: u16,
    pub unk5: u32, // TODO: 0 for ntp3?
    pub caps2: Caps2,
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
    Rgba82 = 17, // TODO: how is this different from type 14?
    BC5Unorm = 22,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"GIDX"))]
pub struct Gidx {
    pub unk1: u32, // 16
    pub hash: u32, // TODO: does this match with material texture hash?
    pub unk3: u32, // 0
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, BinWrite, PartialEq, Clone)]
#[brw(magic(b"eXt\x00"))]
pub struct Ext {
    pub unk1: u32, // 32
    pub unk2: u32, // 16
    pub unk3: u32, // 0
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

#[derive(Debug, Error)]
pub enum CreateNutError {
    #[error("image format {0:?} is not supported")]
    UnsupportedImageFormat(image_dds::ImageFormat),
}

impl NutFormat {
    pub fn block_dim(&self) -> (usize, usize) {
        match self {
            NutFormat::BC1Unorm => (4, 4),
            NutFormat::BC2Unorm => (4, 4),
            NutFormat::BC3Unorm => (4, 4),
            NutFormat::Bgr5A1Unorm => (1, 1),
            NutFormat::Bgr5A1Unorm2 => (1, 1),
            NutFormat::B5G6R5Unorm => (1, 1),
            NutFormat::Rgb5A1Unorm => (1, 1),
            NutFormat::Rgba8Unorm => (1, 1),
            NutFormat::R32Float => (1, 1),
            NutFormat::Rgba82 => (1, 1),
            NutFormat::BC5Unorm => (4, 4),
        }
    }

    pub fn block_size_in_bytes(&self) -> usize {
        match self {
            NutFormat::BC1Unorm => 8,
            NutFormat::BC2Unorm => 16,
            NutFormat::BC3Unorm => 16,
            NutFormat::Bgr5A1Unorm => 2,
            NutFormat::Bgr5A1Unorm2 => 2,
            NutFormat::B5G6R5Unorm => 2,
            NutFormat::Rgb5A1Unorm => 2,
            NutFormat::Rgba8Unorm => 4,
            NutFormat::R32Float => 4,
            NutFormat::Rgba82 => 4,
            NutFormat::BC5Unorm => 16,
        }
    }
}

impl Ntp3 {
    pub fn from_textures_v1<T: AsRef<[u8]>>(
        textures: impl Iterator<Item = (u32, Surface<T>)>,
    ) -> Result<Self, CreateNutError> {
        let textures = textures
            .map(|(hash, surface)| Ntp3TextureV1::from_surface(surface, hash))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            inner: Ntp3Inner::V1(Ntp3InnerV1 {
                count: textures.len() as u16,
                unk2: 0,
                textures,
            }),
        })
    }

    pub fn from_textures_v2<T: AsRef<[u8]>>(
        textures: impl Iterator<Item = (u32, Surface<T>)>,
    ) -> Result<Self, CreateNutError> {
        let textures = textures
            .map(|(hash, surface)| Ntp3TextureV2::from_surface(surface, hash))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            inner: Ntp3Inner::V2(Ntp3InnerV2 {
                count: textures.len() as u16,
                unk2: 0,
                textures,
            }),
        })
    }
}

impl NtwuTexture {
    pub fn deswizzle(&self) -> Result<Vec<u8>, wiiu_swizzle::SwizzleError> {
        if let Some(gtx_header) = &self.gtx_header {
            let mips_start = gtx_header.mipmap_offsets[0] as usize;
            let mips_size = gtx_header.mipmap_data_size as usize;
            if let (Some(image_data), Some(mipmap_data)) = (
                self.data.get(..gtx_header.image_data_size as usize),
                self.data.get(mips_start..mips_start + mips_size),
            ) {
                // TODO: Avoid unwrap.
                wiiu_swizzle::Gx2Surface {
                    dim: wiiu_swizzle::SurfaceDim::from_repr(gtx_header.dim as u32).unwrap(),
                    width: gtx_header.width,
                    height: gtx_header.height,
                    depth_or_array_layers: gtx_header.depth_or_array_layers,
                    mipmap_count: gtx_header.mipmap_count,
                    format: wiiu_swizzle::SurfaceFormat::from_repr(gtx_header.format as u32)
                        .unwrap(),
                    aa: wiiu_swizzle::AaMode::from_repr(gtx_header.aa as u32).unwrap(),
                    usage: gtx_header.usage,
                    image_data,
                    mipmap_data,
                    tile_mode: wiiu_swizzle::TileMode::from_repr(gtx_header.tile_mode as u32)
                        .unwrap(),
                    swizzle: gtx_header.swizzle,
                    alignment: gtx_header.alignment,
                    pitch: gtx_header.pitch,
                    mipmap_offsets: gtx_header.mipmap_offsets,
                }
                .deswizzle()
            } else {
                Err(wiiu_swizzle::SwizzleError::NotEnoughData {
                    expected_size: gtx_header.mipmap_offsets[0] as usize
                        + gtx_header.mipmap_data_size as usize,
                    actual_size: self.data.len(),
                })
            }
        } else {
            // Assume textures without the gtx header aren't tiled.
            Ok(self.data.clone())
        }
    }

    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, CreateSurfaceError> {
        let data = self.deswizzle()?;
        create_surface(&self.header, data)
    }

    pub fn to_dds(&self) -> Result<Dds, image_dds::CreateDdsError> {
        // TODO: Create error type to avoid unwrap.
        self.to_surface().unwrap().to_dds()
    }
}

impl Ntp3TextureV1 {
    // TODO: share code with v2
    pub fn from_surface<T: AsRef<[u8]>>(
        surface: Surface<T>,
        hash: u32,
    ) -> Result<Self, CreateNutError> {
        let (data, unk_sizes) = ntp3_image_data_unk_sizes(&surface);

        let header_size = 80 + unk_sizes.len() as u16 * std::mem::size_of::<u32>() as u16;

        let data_size = surface.data.as_ref().len() as u32;

        // Create an untiled  texture.
        Ok(Self {
            header: TextureHeader {
                size: data_size + header_size as u32,
                unk1: 0,
                data_size,
                header_size,
                unk2: 0,
                unk3: 0,
                mipmap_count: surface.mipmaps as u8,
                unk4: 0,
                format: surface.image_format.try_into()?,
                width: surface.width as u16,
                height: surface.height as u16,
                unk5: 0,
                caps2: if surface.layers == 6 {
                    Caps2::CUBEMAP | Caps2::CUBEMAP_ALLFACES
                } else {
                    Caps2::empty()
                },
            },
            mipmap_data_offset: 0,
            gtx_header: 0,
            unk6: 0,
            unk_sizes,
            ext: Ext {
                unk1: 32,
                unk2: 16,
                unk3: 0,
            },
            gidx: Gidx {
                unk1: 16,
                hash,
                unk3: 0,
            },
            data,
        })
    }

    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, CreateSurfaceError> {
        let data = ntp3_image_data(&self.header, &self.unk_sizes, &self.data);
        create_surface(&self.header, data)
    }

    pub fn to_dds(&self) -> Result<Dds, image_dds::CreateDdsError> {
        // TODO: Create error type to avoid unwrap.
        self.to_surface().unwrap().to_dds()
    }
}

impl Ntp3TextureV2 {
    // TODO: share code with v1
    pub fn from_surface<T: AsRef<[u8]>>(
        surface: Surface<T>,
        hash: u32,
    ) -> Result<Self, CreateNutError> {
        let (data, unk_sizes) = ntp3_image_data_unk_sizes(&surface);

        let header_size = 80 + unk_sizes.len() as u16 * std::mem::size_of::<u32>() as u16;

        let data_size = data.len() as u32;

        // Create an untiled  texture.
        Ok(Self {
            header: TextureHeader {
                size: data_size + header_size as u32,
                unk1: 0,
                data_size,
                header_size,
                unk2: 0,
                unk3: 0,
                mipmap_count: surface.mipmaps as u8,
                unk4: 0,
                format: surface.image_format.try_into()?,
                width: surface.width as u16,
                height: surface.height as u16,
                unk5: 0,
                caps2: if surface.layers == 6 {
                    Caps2::CUBEMAP | Caps2::CUBEMAP_ALLFACES
                } else {
                    Caps2::empty()
                },
            },
            data,
            mipmap_data_offset: 0,
            gtx_header: 0,
            unk6: 0,
            unk_sizes,
            ext: Ext {
                unk1: 32,
                unk2: 16,
                unk3: 0,
            },
            gidx: Gidx {
                unk1: 16,
                hash,
                unk3: 0,
            },
        })
    }

    pub fn to_surface(&self) -> Result<Surface<Vec<u8>>, CreateSurfaceError> {
        let data = ntp3_image_data(&self.header, &self.unk_sizes, &self.data);
        create_surface(&self.header, data)
    }

    pub fn to_dds(&self) -> Result<Dds, image_dds::CreateDdsError> {
        // TODO: Create error type to avoid unwrap.
        self.to_surface().unwrap().to_dds()
    }
}

fn ntp3_image_data_unk_sizes<T: AsRef<[u8]>>(surface: &Surface<T>) -> (Vec<u8>, Vec<u32>) {
    // Each mipmap is aligned to 16 bytes.
    let mut data = Vec::new();
    let mut unk_sizes = Vec::new();
    for layer in 0..surface.layers {
        for mipmap in 0..surface.mipmaps {
            // Each mipmap must be a multiple of 16 bytes.
            let mut mip_data = surface.get(layer, 0, mipmap).unwrap().to_vec();
            mip_data.resize(mip_data.len().next_multiple_of(16), 0);

            if surface.mipmaps > 1 {
                unk_sizes.push(mip_data.len() as u32);
            }

            data.extend_from_slice(&mip_data);
        }
    }

    if surface.image_format == image_dds::ImageFormat::Rgba8Unorm {
        // NTP3 nuts swap the channel order compared to NTWU.
        for pixel in data.chunks_exact_mut(4) {
            if let [r, g, b, a] = *pixel {
                pixel[0] = a;
                pixel[1] = r;
                pixel[2] = g;
                pixel[3] = b;
            }
        }
    }

    if surface.layers == 6 {
        // TODO: Why is this completely different for cubemaps?
        let unk_size = surface.get(0, 0, 0).unwrap().len();
        unk_sizes = vec![unk_size as u32, unk_size as u32];
    }

    // Align to 16 bytes.
    unk_sizes.resize(unk_sizes.len().next_multiple_of(4), 0);

    (data, unk_sizes)
}

fn mip_size(
    width: usize,
    height: usize,
    depth: usize,
    block_width: usize,
    block_height: usize,
    block_depth: usize,
    block_size_in_bytes: usize,
) -> Option<usize> {
    width
        .div_ceil(block_width)
        .checked_mul(height.div_ceil(block_height))
        .and_then(|v| v.checked_mul(depth.div_ceil(block_depth)))
        .and_then(|v| v.checked_mul(block_size_in_bytes))
}

fn create_surface(
    header: &TextureHeader,
    mut image_data: Vec<u8>,
) -> Result<Surface<Vec<u8>>, CreateSurfaceError> {
    if header.format == NutFormat::Rgb5A1Unorm {
        // image_dds only supports Bgr5A1Unorm.
        swap_red_blue_bgr5a1(&mut image_data);
    }

    Ok(Surface {
        width: header.width as u32,
        height: header.height as u32,
        depth: 1,
        layers: if header.caps2 == Caps2::CUBEMAP | Caps2::CUBEMAP_ALLFACES {
            6
        } else {
            1
        },
        mipmaps: header.mipmap_count as u32,
        image_format: header.format.try_into()?,
        data: image_data,
    })
}

fn ntp3_image_data(header: &TextureHeader, unk_sizes: &[u32], image_data: &[u8]) -> Vec<u8> {
    let mut data =
        if unk_sizes.is_empty() || header.caps2 == Caps2::CUBEMAP | Caps2::CUBEMAP_ALLFACES {
            // TODO: How to implement this for cube maps?
            image_data.to_vec()
        } else {
            // Remove mipmap alignment.
            let mut data = Vec::new();
            let (block_width, block_height) = header.format.block_dim();
            let block_size_in_bytes = header.format.block_size_in_bytes();

            let mut offset = 0;
            for (i, size) in unk_sizes
                .iter()
                .enumerate()
                .take(header.mipmap_count as usize)
            {
                let width = mip_dimension(header.width as u32, i as u32);
                let height = mip_dimension(header.height as u32, i as u32);
                let actual_size = mip_size(
                    width as usize,
                    height as usize,
                    1,
                    block_width,
                    block_height,
                    1,
                    block_size_in_bytes,
                )
                .unwrap();
                data.extend_from_slice(&image_data[offset..offset + actual_size]);

                offset += *size as usize;
            }

            data
        };

    if matches!(header.format, NutFormat::Rgba8Unorm | NutFormat::Rgba82) {
        // NTP3 nuts swap the channel order compared to NTWU.
        for pixel in data.chunks_exact_mut(4) {
            if let [a, r, g, b] = *pixel {
                pixel[0] = r;
                pixel[1] = g;
                pixel[2] = b;
                pixel[3] = a;
            }
        }
    }

    data
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
            NutFormat::Rgb5A1Unorm => Ok(image_dds::ImageFormat::Bgr5A1Unorm), // handled by channel swap
            NutFormat::Rgba8Unorm => Ok(image_dds::ImageFormat::Rgba8Unorm),
            NutFormat::R32Float => Ok(image_dds::ImageFormat::R32Float),
            NutFormat::Rgba82 => Ok(image_dds::ImageFormat::Rgba8Unorm),
            NutFormat::BC5Unorm => Ok(image_dds::ImageFormat::BC5RgUnorm),
        }
    }
}

impl TryFrom<image_dds::ImageFormat> for NutFormat {
    type Error = CreateNutError;

    fn try_from(value: image_dds::ImageFormat) -> Result<Self, Self::Error> {
        match value {
            image_dds::ImageFormat::Rgba8Unorm => Ok(NutFormat::Rgba8Unorm),
            image_dds::ImageFormat::R32Float => Ok(NutFormat::R32Float),
            image_dds::ImageFormat::BC1RgbaUnorm => Ok(NutFormat::BC1Unorm),
            image_dds::ImageFormat::BC2RgbaUnorm => Ok(NutFormat::BC2Unorm),
            image_dds::ImageFormat::BC3RgbaUnorm => Ok(NutFormat::BC3Unorm),
            image_dds::ImageFormat::BC5RgUnorm => Ok(NutFormat::BC5Unorm),
            image_dds::ImageFormat::Bgr5A1Unorm => Ok(NutFormat::Bgr5A1Unorm),
            f => Err(CreateNutError::UnsupportedImageFormat(f)),
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

impl Xc3WriteOffsets for Ntp3InnerOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        match self {
            Ntp3InnerOffsets::V1(v1) => {
                v1.write_offsets(writer, base_offset, data_ptr, endian, args)?;
            }
            Ntp3InnerOffsets::V2(v2) => {
                for t in &v2.textures.0 {
                    // NTP3 image data is not tiled and doesn't require alignment.
                    t.data
                        .write_full(writer, t.base_offset, data_ptr, endian, args)?;
                }
            }
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
            *data_ptr = data_ptr.next_multiple_of(4096);
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
