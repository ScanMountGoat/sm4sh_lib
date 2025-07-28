use std::borrow::Cow;

use sm4sh_model::nud::{ImageTexture, NutFormat};
use wgpu::util::DeviceExt;

pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ImageTexture,
) -> wgpu::Texture {
    let (format, data) = image_format_data(texture);

    // TODO: Fix not enough data for mipmaps for some textures.
    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(&format!("{:x}", texture.hash_id)),
            size: wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &data,
    )
}

fn image_format_data(texture: &ImageTexture) -> (wgpu::TextureFormat, Cow<'_, [u8]>) {
    // TODO: Why do final mipmaps not work for some non square textures?
    // Convert unsupported formats to rgba8 for compatibility.
    match texture_format(texture.image_format) {
        Some(format) => (format, Cow::Borrowed(&texture.image_data)),
        None => {
            // TODO: Fix mipmaps for some textures.
            let rgba8 = texture
                .to_surface()
                .decode_layers_mipmaps_rgba8(0..1, 0..1)
                .unwrap_or_else(|_| panic!("{:?}", texture.image_format));
            (wgpu::TextureFormat::Rgba8Unorm, Cow::Owned(rgba8.data))
        }
    }
}

fn texture_format(image_format: NutFormat) -> Option<wgpu::TextureFormat> {
    match image_format {
        NutFormat::BC1Unorm => Some(wgpu::TextureFormat::Bc1RgbaUnorm),
        NutFormat::BC2Unorm => Some(wgpu::TextureFormat::Bc2RgbaUnorm),
        NutFormat::BC3Unorm => Some(wgpu::TextureFormat::Bc3RgbaUnorm),
        NutFormat::Bgr5A1Unorm => None,
        NutFormat::Bgr5A1Unorm2 => None,
        NutFormat::Rgb5A1Unorm => None,
        NutFormat::Rgba8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
        NutFormat::R32Float => Some(wgpu::TextureFormat::R32Float),
        NutFormat::Rgba82 => None,
        NutFormat::BC5Unorm => Some(wgpu::TextureFormat::Bc5RgUnorm),
        NutFormat::B5G6R5Unorm => None,
    }
}
