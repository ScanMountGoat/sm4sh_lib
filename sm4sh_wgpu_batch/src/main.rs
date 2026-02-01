use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use clap::Parser;
use futures::executor::block_on;
use glam::{Mat4, Vec3, Vec4, vec3};
use log::error;
use rayon::prelude::*;
use sm4sh_model::database::ShaderDatabase;
use sm4sh_wgpu::{CameraData, Renderer, SharedData};
use tracing::trace_span;
use wgpu::{
    DeviceDescriptor, Extent3d, PowerPreference, RequestAdapterOptions, TextureDescriptor,
    TextureDimension, TextureUsages,
};

#[cfg(feature = "tracing")]
use tracing_subscriber::prelude::*;

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

fn calculate_camera_data(width: u32, height: u32, translation: Vec3, rotation: Vec3) -> CameraData {
    let aspect = width as f32 / height as f32;

    let view = Mat4::from_translation(translation)
        * Mat4::from_rotation_x(rotation.x)
        * Mat4::from_rotation_y(rotation.y);

    let projection = Mat4::perspective_rh(FOV_Y, aspect, Z_NEAR, Z_FAR);

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
    /// The shader database file
    database: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Check for any errors.
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .with_module_level("sm4sh_wgpu", log::LevelFilter::Warn)
        .init()?;

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .unwrap();

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

    let database = ShaderDatabase::from_file(&cli.database)?;
    let shared_data = SharedData::new(&device, &queue, database);

    // Load and render folders individually to save on memory.
    let root_folder = Path::new(&cli.root_folder);

    // Render only the main nud model from each folder.
    let start = std::time::Instant::now();
    let paths: Vec<_> = globwalk::GlobWalkerBuilder::from_patterns(root_folder, &["*model.nud"])
        .build()?
        .filter_map(Result::ok)
        .map(|e| e.path().to_path_buf())
        .collect();

    let renderer = Mutex::new(Renderer::new(&device, WIDTH, HEIGHT, surface_format));

    paths.par_iter().for_each(|path| {
        let nud_model = sm4sh_model::load_model(path);

        match nud_model {
            Ok(nud_model) => {
                let model = sm4sh_wgpu::load_model(&device, &queue, &nud_model, &shared_data);

                let output_path = screenshot_path(root_folder, path);

                // Create a unique buffer to avoid mapping a buffer from multiple threads.
                let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    size: WIDTH as u64 * HEIGHT as u64 * 4,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    label: None,
                    mapped_at_creation: false,
                });

                let span = trace_span!("render_model");
                span.in_scope(|| {
                    // Each model updates the renderer's internal buffers for camera framing.
                    // We need to hold the lock until the output image has been copied to the buffer.
                    // Rendering is cheap, so this has little performance impact in practice.
                    let renderer = renderer.lock().unwrap();

                    // Initialize the camera to frame the model.
                    let camera = frame_bounds(model.bounding_sphere);
                    renderer.update_camera(&queue, &camera);

                    // Render to a buffer to save as PNG.
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                    renderer.render_model(&mut encoder, &output_view, &model, &camera);

                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            aspect: wgpu::TextureAspect::All,
                            texture: &output,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &output_buffer,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(WIDTH * 4),
                                rows_per_image: Some(HEIGHT),
                            },
                        },
                        texture_desc.size,
                    );
                    queue.submit([encoder.finish()]);
                });

                save_screenshot(&device, &output_buffer, output_path);
            }
            Err(e) => {
                error!("Error loading {path:?}: {e}");
            }
        }
    });

    println!("Completed in {:?}", start.elapsed());
    Ok(())
}

fn screenshot_path(root_folder: &Path, path: &Path) -> PathBuf {
    // Convert data/fighter/mario/model/body/c00/model.nud to fighter_mario_model_body_c00.
    let output_path = path
        .parent()
        .unwrap()
        .strip_prefix(root_folder)
        .unwrap()
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("_");
    root_folder.join(output_path).with_extension("png")
}

#[tracing::instrument(skip_all)]
fn save_screenshot(device: &wgpu::Device, output_buffer: &wgpu::Buffer, output_path: PathBuf) {
    // Save the output texture.
    // Adapted from WGPU Example https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/capture
    {
        let buffer_slice = output_buffer.slice(..);

        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        block_on(rx.receive()).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let mut buffer = image::RgbaImage::from_raw(WIDTH, HEIGHT, data.to_owned()).unwrap();
        // Force opaque to match sm4sh_viewer.
        buffer.pixels_mut().for_each(|rgba| rgba[3] = 255u8);
        buffer.save(output_path).unwrap();
    }
    output_buffer.unmap();
}

fn frame_bounds(bounding_sphere: Vec4) -> CameraData {
    // Find the base of the triangle based on vertical FOV and bounding sphere "height".
    // The aspect ratio is 1.0, so FOV_X is also FOV_Y.
    let distance = bounding_sphere.w / FOV_Y.tan() * 2.0;
    let translation = vec3(bounding_sphere.x, -bounding_sphere.y, -distance);
    let rotation = Vec3::ZERO;
    calculate_camera_data(WIDTH, HEIGHT, translation, rotation)
}
