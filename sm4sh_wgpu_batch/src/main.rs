use std::path::Path;

use clap::Parser;
use futures::executor::block_on;
use log::error;
use sm4sh_model::database::ShaderDatabase;
use sm4sh_wgpu::{CameraData, Model, Renderer, SharedData};
use wgpu::{
    DeviceDescriptor, Extent3d, PowerPreference, RequestAdapterOptions, TextureDescriptor,
    TextureDimension, TextureUsages,
};

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

fn calculate_camera_data(
    width: u32,
    height: u32,
    translation: glam::Vec3,
    rotation: glam::Vec3,
) -> CameraData {
    let aspect = width as f32 / height as f32;

    let view = glam::Mat4::from_translation(translation)
        * glam::Mat4::from_rotation_x(rotation.x)
        * glam::Mat4::from_rotation_y(rotation.y);

    let projection = glam::Mat4::perspective_rh(FOV_Y, aspect, Z_NEAR, Z_FAR);

    let view_projection = projection * view;

    let position = view.inverse().col(3);

    CameraData {
        view,
        projection,
        view_projection,
        position,
        width,
        height,
    }
}

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The source folder to search recursively for models and save the final PNG renders.
    root_folder: String,
    /// The shader database JSON file
    database: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Check for any errors.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .with_module_level("sm4sh_wgpu", log::LevelFilter::Warn)
        .init()?;

    // Load models in headless mode without a surface.
    // This simplifies testing for stability and performance.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        ..Default::default()
    }))?;
    let (device, queue) = block_on(adapter.request_device(&DeviceDescriptor {
        required_features: sm4sh_wgpu::FEATURES,
        ..Default::default()
    }))?;

    let surface_format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&device, WIDTH, HEIGHT, surface_format);

    // TODO: Frame each model individually?

    let camera = calculate_camera_data(
        WIDTH,
        HEIGHT,
        glam::vec3(0.0, -8.0, -60.0),
        glam::Vec3::ZERO,
    );
    renderer.update_camera(&queue, &camera);

    let texture_desc = TextureDescriptor {
        size: Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: surface_format,
        usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };
    let output = device.create_texture(&texture_desc);
    let output_view = output.create_view(&Default::default());

    let database = ShaderDatabase::from_file(&cli.database);
    let shared_data = SharedData::new(&device, database);

    // Load and render folders individually to save on memory.
    let root_folder = Path::new(&cli.root_folder);

    // Render each model folder.
    let start = std::time::Instant::now();
    let paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(root_folder, &["*.{nud}"])
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect();

    // Round up to avoid skipping any files at the end.
    let n = paths.len().div_ceil(rayon::current_num_threads());

    // Rayon's thread pool causes weird texture rendering issues potentially due to work stealing.
    // TODO: Investigate why textures don't load properly when using Rayon's threadpool.
    // Scoped threads are slightly less efficient but don't have this issue.
    std::thread::scope(|s| {
        for i in 0..rayon::current_num_threads() {
            let paths = paths.iter().skip(i * n).take(n);
            s.spawn(|| {
                for path in paths {
                    let nud_model = sm4sh_model::load_model(path);

                    match nud_model {
                        Ok(nud_model) => {
                            let model =
                                sm4sh_wgpu::load_model(&device, &queue, &nud_model, &shared_data);

                            // Create a unique buffer to avoid mapping a buffer from multiple threads.
                            let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                                size: WIDTH as u64 * HEIGHT as u64 * 4,
                                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                                label: None,
                                mapped_at_creation: false,
                            });

                            // Convert fighter/mario/model/body/c00/model.nud to mario_model_body_c00.
                            let output_path = path
                                .parent()
                                .unwrap()
                                .strip_prefix(root_folder)
                                .unwrap()
                                .components()
                                .map(|c| c.as_os_str().to_string_lossy())
                                .collect::<Vec<_>>()
                                .join("_");
                            let output_path = root_folder.join(output_path).with_extension("png");

                            render_screenshot(
                                &device,
                                &renderer,
                                &output_view,
                                &model,
                                &camera,
                                &output,
                                &output_buffer,
                                texture_desc.size,
                                &queue,
                                output_path,
                            );
                        }
                        Err(e) => {
                            error!("Error loading {path:?}: {e}");
                        }
                    }
                }
            });
        }
    });

    println!("Completed in {:?}", start.elapsed());
    Ok(())
}

fn render_screenshot(
    device: &wgpu::Device,
    renderer: &Renderer,
    output_view: &wgpu::TextureView,
    model: &Model,
    camera: &CameraData,
    output: &wgpu::Texture,
    output_buffer: &wgpu::Buffer,
    size: wgpu::Extent3d,
    queue: &wgpu::Queue,
    output_path: std::path::PathBuf,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    renderer.render_model(&mut encoder, output_view, model, camera);

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture: output,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);

    // Save the output texture.
    // Adapted from WGPU Example https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/capture
    {
        let buffer_slice = output_buffer.slice(..);

        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::PollType::Wait).unwrap();
        block_on(rx.receive()).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let mut buffer = image::RgbaImage::from_raw(WIDTH, HEIGHT, data.to_owned()).unwrap();
        // Force opaque to match sm4sh_viewer.
        buffer.pixels_mut().for_each(|rgba| rgba[3] = 255u8);
        buffer.save(output_path).unwrap();
    }
    output_buffer.unmap();
}
