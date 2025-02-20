use std::borrow::Cow;

use sm4sh_model::nud::{ImageTexture, NutFormat};
use wgpu::util::DeviceExt;

pub fn create_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ImageTexture,
) -> wgpu::Texture {
    let (format, data) = image_format_data(texture);

    device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(&format!("{:x}", texture.hash_id)),
            size: wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: texture.mipmap_count,
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

fn image_format_data(texture: &ImageTexture) -> (wgpu::TextureFormat, Cow<'_, Vec<u8>>) {
    // TODO: Why do final mipmaps not work for some non square textures?
    let mut data = texture.image_data.clone();
    data.resize(data.len() + 32, 0u8);

    // Convert unsupported formats to rgba8 for compatibility.
    match texture_format(texture.image_format) {
        Some(format) => (format, Cow::Owned(data)),
        None => {
            let rgba8 = texture.to_surface().decode_rgba8().unwrap();
            (wgpu::TextureFormat::Rgba8Unorm, Cow::Owned(rgba8.data))
        }
    }
}

fn texture_format(image_format: NutFormat) -> Option<wgpu::TextureFormat> {
    match image_format {
        NutFormat::BC1Unorm => Some(wgpu::TextureFormat::Bc1RgbaUnorm),
        NutFormat::BC2Unorm => Some(wgpu::TextureFormat::Bc2RgbaUnorm),
        NutFormat::BC3Unorm => Some(wgpu::TextureFormat::Bc3RgbaUnorm),
        NutFormat::Unk6 => None,
        NutFormat::Rg16 => Some(wgpu::TextureFormat::Bc1RgbaUnorm),
        NutFormat::Rgb5A1Unorm => None, // channel swapping handled in sm4sh_lib
        NutFormat::Rgba8 => Some(wgpu::TextureFormat::Rgba8Unorm),
        NutFormat::Bgra8 => Some(wgpu::TextureFormat::Bgra8Unorm),
        NutFormat::Rgba82 => None,
        NutFormat::BC5Unorm => Some(wgpu::TextureFormat::Bc5RgUnorm),
    }
}
