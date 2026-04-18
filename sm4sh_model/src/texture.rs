use image_dds::image;
use sm4sh_lib::nut::NutFormat;

use crate::ImageTexture;

pub fn global_textures() -> Vec<ImageTexture> {
    vec![
        solid_color_texture(0x10000001, [238, 28, 36, 255]),
        solid_color_texture(0x10000007, [255, 255, 255, 255]),
        load_png_cube_image(0x10000008, include_bytes!("images/10000008.png")),
        solid_color_texture(0x10080000, [255, 255, 255, 255]),
        load_png_cube_image(0x10101000, include_bytes!("images/10101000.png")),
        load_png_cube_image(0x10102000, include_bytes!("images/10102000.png")),
        solid_color_texture(0x10104100, [238, 28, 36, 255]),
        solid_color_texture(0x10104FFF, [0, 0, 0, 0]),
    ]
}

fn solid_color_texture(hash_id: u32, rgba: [u8; 4]) -> ImageTexture {
    ImageTexture {
        hash_id,
        width: 4,
        height: 4,
        mipmap_count: 1,
        layers: 1,
        image_format: NutFormat::Rgba8Unorm,
        image_data: std::iter::repeat_n(rgba, 4 * 4).flatten().collect(),
    }
}

fn load_png_cube_image(hash_id: u32, bytes: &[u8]) -> ImageTexture {
    // TODO: methods for to_surface for rgba8 and rgbaf32?
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap();
    ImageTexture {
        hash_id,
        width: image.width(),
        height: image.height() / 6,
        mipmap_count: 1,
        layers: 6,
        image_format: NutFormat::Rgba8Unorm,
        image_data: image.into_rgba8().into_raw(),
    }
}
