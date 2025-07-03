use std::path::Path;

use anyhow::Context;
use clap::Parser;
use futures::executor::block_on;
use glam::{vec3, Vec3};
use log::{error, info};
use sm4sh_lib::{nud::Nud, nut::Nut};
use sm4sh_model::nud::NudModel;
use sm4sh_wgpu::{load_model, CameraData, Model, Renderer};
use winit::{
    dpi::PhysicalPosition,
    event::*,
    event_loop::EventLoop,
    keyboard::NamedKey,
    window::{Window, WindowBuilder},
};

const FOV_Y: f32 = 0.5;
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 100000.0;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    // Camera
    translation: Vec3,
    rotation_xyz: Vec3,
    camera: CameraData,

    renderer: Renderer,

    model: Model,

    input_state: InputState,
}

#[derive(Default)]
struct InputState {
    is_mouse_left_clicked: bool,
    is_mouse_right_clicked: bool,
    previous_cursor_position: PhysicalPosition<f64>,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window, cli: &Cli) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        info!("{:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: sm4sh_wgpu::FEATURES,
                ..Default::default()
            })
            .await
            .unwrap();

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, size.width, size.height, config.format);

        // Initialize the camera transform.
        let translation = vec3(0.0, -8.0, -50.0);
        let rotation_xyz = Vec3::ZERO;
        let camera = calculate_camera_data(size, translation, rotation_xyz);
        renderer.update_camera(&queue, &camera);

        let path = Path::new(&cli.file);
        let nud = Nud::from_file(path)?;
        let nut = Nut::from_file(path.with_file_name("model.nut"))?;
        let nud_model = NudModel::from_nud(&nud, &nut)?;
        let model = load_model(&device, &queue, &nud_model, config.format);

        Ok(Self {
            surface,
            device,
            queue,
            size,
            config,
            translation,
            rotation_xyz,
            camera,
            renderer,
            model,
            input_state: Default::default(),
        })
    }

    fn update_camera(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let camera = calculate_camera_data(size, self.translation, self.rotation_xyz);
        self.camera = camera;
        self.renderer.update_camera(&self.queue, &camera);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // Update each resource that depends on window size.
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.renderer
                .resize(&self.device, new_size.width, new_size.height);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.renderer
            .render_model(&mut encoder, &output_view, &self.model, &self.camera);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    // Make this a reusable library that only requires glam?
    fn handle_input(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::Key::Named(named) = &event.logical_key {
                    match named {
                        // Basic camera controls using arrow keys.
                        NamedKey::ArrowLeft => self.translation.x += 0.1,
                        NamedKey::ArrowRight => self.translation.x -= 0.1,
                        NamedKey::ArrowUp => self.translation.y -= 0.1,
                        NamedKey::ArrowDown => self.translation.y += 0.1,
                        _ => (),
                    }
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                // Track mouse clicks to only rotate when dragging while clicked.
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.input_state.is_mouse_left_clicked = true
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.input_state.is_mouse_left_clicked = false
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.input_state.is_mouse_right_clicked = true
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.input_state.is_mouse_right_clicked = false
                    }
                    _ => (),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input_state.is_mouse_left_clicked {
                    let delta_x = position.x - self.input_state.previous_cursor_position.x;
                    let delta_y = position.y - self.input_state.previous_cursor_position.y;

                    // Swap XY so that dragging left/right rotates left/right.
                    self.rotation_xyz.x += (delta_y * 0.01) as f32;
                    self.rotation_xyz.y += (delta_x * 0.01) as f32;
                } else if self.input_state.is_mouse_right_clicked {
                    let delta_x = position.x - self.input_state.previous_cursor_position.x;
                    let delta_y = position.y - self.input_state.previous_cursor_position.y;

                    // Translate an equivalent distance in screen space based on the camera.
                    // The viewport height and vertical field of view define the conversion.
                    let fac = FOV_Y.sin() * self.translation.z.abs() / self.size.height as f32;

                    // Negate y so that dragging up "drags" the model up.
                    self.translation.x += delta_x as f32 * fac;
                    self.translation.y -= delta_y as f32 * fac;
                }
                // Always update the position to avoid jumps when moving between clicks.
                self.input_state.previous_cursor_position = *position;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Scale zoom speed with distance to make it easier to zoom out large scenes.
                let delta_z = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => *y * self.translation.z.abs() * 0.1,
                    MouseScrollDelta::PixelDelta(p) => {
                        p.y as f32 * self.translation.z.abs() * 0.005
                    }
                };

                self.translation.z += delta_z;
            }
            _ => (),
        }
    }
}

fn calculate_camera_data(
    size: winit::dpi::PhysicalSize<u32>,
    translation: glam::Vec3,
    rotation: glam::Vec3,
) -> CameraData {
    let aspect = size.width as f32 / size.height as f32;

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
        width: size.width,
        height: size.height,
    }
}

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    /// The nud file
    file: String,
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("wgpu", log::LevelFilter::Warn)
        .with_module_level("naga", log::LevelFilter::Warn)
        .init()
        .unwrap();

    let cli = Cli::parse();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title(concat!("sm4sh_viewer ", env!("CARGO_PKG_VERSION")))
        .build(&event_loop)
        .unwrap();

    let mut state = block_on(State::new(&window, &cli))?;
    event_loop
        .run(|event, target| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                    state.update_camera(*physical_size);
                    window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {}
                WindowEvent::RedrawRequested => {
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                        Err(e) => error!("{e:?}"),
                    }
                    window.request_redraw();
                }
                _ => {
                    state.handle_input(event);
                    state.update_camera(window.inner_size());
                    window.request_redraw();
                }
            },
            _ => (),
        })
        .with_context(|| "failed to complete event loop")?;
    Ok(())
}
