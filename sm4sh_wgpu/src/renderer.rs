use glam::{Mat4, UVec4, Vec4, vec4};

use crate::{CameraData, DeviceBufferExt, Model, QueueBufferExt, skeleton::BoneRenderer};

// TODO: Change these formats for better compatibility.
pub(crate) const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Snorm;
pub(crate) const BLOOM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg11b10Ufloat;
pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(crate) const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(crate) const VARIANCE_SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Unorm;

pub struct Renderer {
    camera_buffer: wgpu::Buffer,
    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,
    textures: Textures,
    bone_renderer: BoneRenderer,
    fb0_buffer: wgpu::Buffer,
    fb1_buffer: wgpu::Buffer,

    bloom_bright_pipeline: wgpu::RenderPipeline,
    bloom_add_pipeline: wgpu::RenderPipeline,
    bloom_blur_combine_pipeline: wgpu::RenderPipeline,
    bloom_blur_pipeline: wgpu::RenderPipeline,

    blit_pipeline: wgpu::RenderPipeline,

    variance_shadow_pipeline: wgpu::RenderPipeline,

    model_shadow_depth_pipeline: wgpu::RenderPipeline,

    shadow_map: wgpu::TextureView,
    variance_shadow_map: wgpu::TextureView,
    variance_shadow_bind_group: crate::shader::variance_shadow::bind_groups::BindGroup0,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        let camera = CameraData {
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
            view_projection: Mat4::IDENTITY,
            position: Vec4::ZERO,
            width,
            height,
        };
        let camera_buffer = device.create_uniform_buffer("camera buffer", &camera.to_shader_data());

        // Default values for all buffers taken from Rosalina c00 on Miiverse stage.
        let fb0_buffer = device.create_uniform_buffer("FB0", &fb0(width as f32, height as f32));
        let fb1_buffer = device.create_uniform_buffer("FB1", &fb1());
        let fb3_buffer = device.create_uniform_buffer(
            "FB3",
            &crate::shader::model::Fb3 {
                hdr_range: vec4(0.5, 2.0, 0.0, 0.0),
                colr_hdr_range: Vec4::ZERO,
            },
        );
        let fb4_buffer = device.create_uniform_buffer(
            "FB4",
            &crate::shader::model::Fb4 {
                effect_light_entry: Vec4::ZERO,
            },
        );
        let fb5_buffer = device.create_uniform_buffer(
            "FB5",
            &crate::shader::model::Fb5 {
                effect_light_area: UVec4::ZERO,
            },
        );

        let shadow_map = create_texture(device, 1024, 1024, "shadow map", SHADOW_FORMAT);
        let variance_shadow_map = create_texture(
            device,
            512,
            512,
            "variance shadow map",
            VARIANCE_SHADOW_FORMAT,
        );
        let depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let variance_shadow_bind_group =
            crate::shader::variance_shadow::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::variance_shadow::bind_groups::BindGroupLayout0 {
                    depth: &shadow_map,
                    depth_sampler: &depth_sampler,
                },
            );

        // g_VSMTextureSampler in shaders.
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToBorder,
            address_mode_v: wgpu::AddressMode::ClampToBorder,
            address_mode_w: wgpu::AddressMode::ClampToBorder,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            border_color: Some(wgpu::SamplerBorderColor::OpaqueWhite),
            ..Default::default()
        });
        let model_bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
                fb0: fb0_buffer.as_entire_buffer_binding(),
                fb1: fb1_buffer.as_entire_buffer_binding(),
                fb3: fb3_buffer.as_entire_buffer_binding(),
                fb4: fb4_buffer.as_entire_buffer_binding(),
                fb5: fb5_buffer.as_entire_buffer_binding(),
                g_vsm_texture: &variance_shadow_map,
                g_vsm_sampler: &shadow_sampler,
            },
        );

        let textures = Textures::new(device, width, height);

        let bone_renderer = BoneRenderer::new(device, &camera_buffer, COLOR_FORMAT);

        let bloom_add_pipeline = bloom_add_pipeline(device, COLOR_FORMAT);
        let bloom_blur_combine_pipeline = bloom_blur_combine_pipeline(device, BLOOM_FORMAT);
        let bloom_blur_pipeline = bloom_blur_pipeline(device, BLOOM_FORMAT);
        let bloom_bright_pipeline = bloom_bright_pipeline(device, BLOOM_FORMAT);
        let blit_pipeline = blit_pipeline(device, output_format);
        let variance_shadow_pipeline = variance_shadow_pipeline(device);
        let model_shadow_depth_pipeline = model_shadow_depth_pipeline(device);

        Self {
            camera_buffer,
            model_bind_group0,
            textures,
            bone_renderer,
            fb0_buffer,
            fb1_buffer,
            blit_pipeline,
            bloom_add_pipeline,
            bloom_bright_pipeline,
            bloom_blur_combine_pipeline,
            bloom_blur_pipeline,
            variance_shadow_pipeline,
            model_shadow_depth_pipeline,
            shadow_map,
            variance_shadow_map,
            variance_shadow_bind_group,
        }
    }

    pub fn render_model(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        model: &Model,
        camera: &CameraData,
    ) {
        self.model_shadow_depth_pass(encoder, model);
        self.variance_shadow_pass(encoder);
        self.model_pass(encoder, model, camera);
        self.bloom_bright_pass(encoder);
        self.bloom_blur_pass(
            encoder,
            &self.textures.bloom_blur1,
            &self.textures.bloom_blur1_bindgroup,
        );
        self.bloom_blur_pass(
            encoder,
            &self.textures.bloom_blur2,
            &self.textures.bloom_blur2_bindgroup,
        );
        self.bloom_blur_pass(
            encoder,
            &self.textures.bloom_blur3,
            &self.textures.bloom_blur3_bindgroup,
        );
        self.bloom_blur_pass(
            encoder,
            &self.textures.bloom_blur4,
            &self.textures.bloom_blur4_bindgroup,
        );
        self.bloom_blur_combine_pass(encoder);
        self.bloom_add_pass(encoder);
        self.blit_pass(encoder, output_view);
    }

    fn model_shadow_depth_pass(&self, encoder: &mut wgpu::CommandEncoder, model: &Model) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Shadow Depth Pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_map,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.model_bind_group0.set(&mut pass);
        model.draw_shadow_depth(&mut pass, &self.model_shadow_depth_pipeline);
    }

    fn variance_shadow_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Variance Shadow Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.variance_shadow_map,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.variance_shadow_pipeline);
        crate::shader::variance_shadow::set_bind_groups(
            &mut pass,
            &self.variance_shadow_bind_group,
        );
        pass.draw(0..3, 0..1);
    }

    fn model_pass(&self, encoder: &mut wgpu::CommandEncoder, model: &Model, camera: &CameraData) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.textures.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.model_bind_group0.set(&mut pass);
        model.draw(&mut pass, camera);

        self.bone_renderer
            .draw_bones(&mut pass, &model.bone_transforms, model.bone_count);
    }

    fn bloom_bright_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom Bright Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.bloom_bright,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.bloom_bright_pipeline);
        crate::shader::blit::set_bind_groups(&mut pass, &self.textures.blit_bind_group);
        pass.draw(0..3, 0..1);
    }

    fn bloom_blur_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        bind_group: &crate::shader::bloom_blur::bind_groups::BindGroup0,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom Blur Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.bloom_blur_pipeline);
        crate::shader::bloom_blur::set_bind_groups(&mut pass, bind_group);
        pass.draw(0..3, 0..1);
    }

    fn bloom_blur_combine_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom Blur Combine Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.bloom_blur_combined,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.bloom_blur_combine_pipeline);
        crate::shader::bloom_blur_combine::set_bind_groups(
            &mut pass,
            &self.textures.bloom_blur_combine_bindgroup,
        );
        pass.draw(0..3, 0..1);
    }

    fn bloom_add_pass(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom Add Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.textures.color,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.bloom_add_pipeline);
        crate::shader::bloom_add::set_bind_groups(&mut pass, &self.textures.bloom_add_bindgroup);
        pass.draw(0..3, 0..1);
    }

    fn blit_pass(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Blit Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.blit_pipeline);
        crate::shader::blit::set_bind_groups(&mut pass, &self.textures.blit_bind_group);
        pass.draw(0..3, 0..1);
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &CameraData) {
        queue.write_uniform_data(&self.camera_buffer, &camera.to_shader_data());
    }

    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        // Update each resource that depends on window size.
        self.textures = Textures::new(device, width, height);
        queue.write_uniform_data(&self.fb0_buffer, &fb0(width as f32, height as f32));
    }
}

fn bloom_add_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::bloom_add::create_shader_module(device);
    let layout = crate::shader::bloom_add::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Bloom Add Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::bloom_add::vertex_state(
            &module,
            &crate::shader::bloom_add::vs_main_entry(),
        ),
        fragment: Some(crate::shader::bloom_add::fragment_state(
            &module,
            &crate::shader::bloom_add::fs_main_entry([Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::all(),
            })]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn bloom_blur_combine_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = crate::shader::bloom_blur_combine::create_shader_module(device);
    let layout = crate::shader::bloom_blur_combine::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Bloom Blur Combine Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::bloom_blur_combine::vertex_state(
            &module,
            &crate::shader::bloom_blur_combine::vs_main_entry(),
        ),
        fragment: Some(crate::shader::bloom_blur_combine::fragment_state(
            &module,
            &crate::shader::bloom_blur_combine::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn bloom_blur_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::bloom_blur::create_shader_module(device);
    let layout = crate::shader::bloom_blur::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Bloom Blur Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::bloom_blur::vertex_state(
            &module,
            &crate::shader::bloom_blur::vs_main_entry(),
        ),
        fragment: Some(crate::shader::bloom_blur::fragment_state(
            &module,
            &crate::shader::bloom_blur::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn bloom_bright_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = crate::shader::bloom_bright::create_shader_module(device);
    let layout = crate::shader::bloom_bright::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Bloom Bright Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::bloom_bright::vertex_state(
            &module,
            &crate::shader::bloom_bright::vs_main_entry(),
        ),
        fragment: Some(crate::shader::bloom_bright::fragment_state(
            &module,
            &crate::shader::bloom_bright::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn blit_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = crate::shader::blit::create_shader_module(device);
    let layout = crate::shader::blit::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Blit Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::blit::vertex_state(&module, &crate::shader::blit::vs_main_entry()),
        fragment: Some(crate::shader::blit::fragment_state(
            &module,
            &crate::shader::blit::fs_main_entry([Some(format.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn variance_shadow_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::variance_shadow::create_shader_module(device);
    let layout = crate::shader::variance_shadow::create_pipeline_layout(device);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Variance Shadow Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::variance_shadow::vertex_state(
            &module,
            &crate::shader::variance_shadow::vs_main_entry(),
        ),
        fragment: Some(crate::shader::variance_shadow::fragment_state(
            &module,
            &crate::shader::variance_shadow::fs_main_entry([Some(VARIANCE_SHADOW_FORMAT.into())]),
        )),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn model_shadow_depth_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = crate::shader::model::create_shader_module(device);
    let layout = crate::shader::model::create_pipeline_layout(device);

    // TODO: does the shadow depth pipeline need to be mesh specific?
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Model Shadow Depth Pipeline"),
        layout: Some(&layout),
        vertex: crate::shader::model::vertex_state(
            &module,
            &crate::shader::model::vs_shadow_entry(wgpu::VertexStepMode::Vertex),
        ),
        fragment: None,
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: SHADOW_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn fb0(width: f32, height: f32) -> crate::shader::model::Fb0 {
    crate::shader::model::Fb0 {
        depth_of_field0: vec4(0.0, 0.0, 0.0, 0.0),
        depth_of_field1: vec4(0.0, 0.0, 0.0, 0.0),
        depth_of_field_tex_size: vec4(0.0, 0.0, 0.0, 0.0),
        proj_inv_matrix: Mat4::IDENTITY, // TODO: Fill in this value
        refraction_param: vec4(0.0, 0.0, 0.0, 0.0),
        proj_to_view: vec4(0.47635, 0.26795, 256.00, 0.00),
        view_to_proj: vec4(1.04964, -1.86603, 0.00391, 0.00),
        gi_buffer_size: vec4(width / 4.0, height / 4.0, 4.0 / width, 4.0 / height),
        weight0: vec4(0.14374, 0.1258, 0.09635, 0.06459),
        weight1: vec4(0.03789, 0.01945, 0.00874, 0.00344),
        random_vector: [Vec4::ZERO; 31], // TODO: Fill in these values
        reflection_param: vec4(0.0075, 2.50, 0.25, 0.00),
        sun_shaft_light_param0: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        sun_shaft_light_param1: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        sun_shaft_blur_param: [
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
        ],
        sun_shaft_composite_param: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        glare_abstract_param: vec4(1.0, 4.0, 0.0, 0.0),
        render_target_tex_size: vec4(1.0 / width, 1.0 / height, 2.0 / width, 2.0 / height),
        glare_fog_param: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        glare_simple_color: vec4(0.0, 0.0, 0.0, 0.0),
        pad0_fb0: vec4(0.0, 0.0, 0.0, 0.0),
        lens_flare_param: vec4(0.0, 0.0, 0.0, 0.0),
        outline_param: vec4(0.25, 0.00, 0.00, 0.00),
        post_reflection_color: vec4(0.50, 0.50, 0.50, 0.20),
        multi_shadow_matrix: [Mat4::IDENTITY; 4], // TODO: fill in these values
        shadow_map_matrix: Mat4::from_cols_array_2d(&[
            [0.00814, 0.00, 0.00, 0.00],
            [0.00, -0.00504, -0.01631, 0.00],
            [0.00, 0.01385, -0.00594, 0.00],
            [0.49189, 0.67917, 1.09728, 1.00],
        ]), // TODO: fill in these values
        view: Mat4::ZERO,                         // TODO: fill in these values
        eye: vec4(40.0, 47.40689, 37.02085, 1.0), // TODO: fill in these values
        constant_color: vec4(1.0, 1.0, 1.0, 1.0),
        light_map_pos: vec4(0.0, 0.0, 0.0, 0.0),
        reflection_gain: vec4(1.0, 1.0, 1.0, 1.0),
        hdr_constant: vec4(0.5, 2.0, 1.0, 1.0),
        _g_fresnel_color: vec4(1.0, 1.0, 1.0, 1.0),
        effect_light_param0: vec4(0.1, 0.1, -15.0, 0.0),
        effect_light_param1: vec4(30.0, 12.0, 29.0, 11.0),
        bg_rot_inv: Mat4::IDENTITY,
        reflection_color1: vec4(0.0, 0.0, 0.0, 0.0),
        reflection_color2: vec4(0.0001, 0.0, 0.0, 0.0),
        reflection_color3: vec4(0.315, 0.31792, 0.35, 1.0),
        effect_light_param2: vec4(0.685, 0.68208, 0.65, 1.00),
    }
}

fn fb1() -> crate::shader::model::Fb1 {
    crate::shader::model::Fb1 {
        light_map_matrix: Mat4::IDENTITY,
        blink_color: vec4(1.0, 1.0, 1.0, 0.0),
        g_constant_volume: vec4(1.0, 1.0, 1.0, 1.0),
        g_constant_offset: vec4(0.0, 0.0, 0.0, 0.0),
        uv_scroll_counter: vec4(0.35, 0.0, 0.0, 0.0), // TODO: changes over time?
        spycloak_params: vec4(-100.0, 0.0, 0.0, 0.0),
        compress_param: vec4(1.0, 0.0, 0.0, 0.0),
        g_fresnel_color: vec4(1.0, 1.0, 1.0, 1.0),
        depth_offset: vec4(0.0, 0.0, 0.0, 0.0),
        outline_color: vec4(0.0, 0.0, 0.0, 1.0),
        pad0_fb1: [
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
        ],
        light_map_color_gain: vec4(0.4875, 0.4875, 0.4875, 0.0),
        light_map_color_offset: vec4(0.0, 0.0, 0.0, 0.0),
        ceiling_dir: vec4(0.0, 1.0, 0.0, 0.0),
        ceiling_color: vec4(0.15, 0.15, 0.15, 0.0),
        ground_color: vec4(1.0, 1.0, 1.0, 0.0),
        ambient_color: vec4(0.0, 0.0, 0.0, 0.0),
        light_dir_color1: vec4(0.75, 0.75, 0.75, 0.0),
        light_dir_color2: vec4(0.2, 0.2, 0.2, 1.0),
        light_dir_color3: vec4(0.0, 0.0, 0.0, 0.0),
        light_dir1: vec4(0.0, -0.84323, -0.53756, 0.0),
        light_dir2: vec4(-0.87287, 0.43644, -0.21822, 0.0),
        light_dir3: vec4(0.0, 0.0, 0.0, 0.0),
        fog_color: vec4(1.0, 1.0, 1.0, 1.0),
        g_fresnel_offset: vec4(0.0, 0.0, 0.0, 0.0),
        shadow_map_param: vec4(0.001, 0.0, 0.0, 0.0),
        char_shadow_color: vec4(0.315, 0.31792, 0.35, 1.0),
        char_shadow_color2: vec4(0.685, 0.68208, 0.65, 1.0),
        soft_lighting_params2: vec4(0.0, 0.0, 0.0, 1.0),
        bg_shadow_color: vec4(0.81, 0.8175, 0.90, 1.0),
        g_ibl_color_gain: vec4(1.0, 1.0, 1.0, 0.0),
        g_ibl_color_offset: vec4(0.15, 0.15, 0.15, 0.0),
        g_constant_min: Vec4::ZERO,
        loupe_shadow_params: Vec4::ZERO,
        soft_light_color_gain: Vec4::ZERO,
        soft_light_color_offset: Vec4::ZERO,
        character_color: Vec4::ZERO,
    }
}

// Group resizable resources to avoid duplicating this logic.
pub struct Textures {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,

    blit_bind_group: crate::shader::blit::bind_groups::BindGroup0,

    bloom_add_bindgroup: crate::shader::bloom_add::bind_groups::BindGroup0,

    bloom_bright: wgpu::TextureView,

    bloom_blur1: wgpu::TextureView,
    bloom_blur1_bindgroup: crate::shader::bloom_blur::bind_groups::BindGroup0,

    bloom_blur2: wgpu::TextureView,
    bloom_blur2_bindgroup: crate::shader::bloom_blur::bind_groups::BindGroup0,

    bloom_blur3: wgpu::TextureView,
    bloom_blur3_bindgroup: crate::shader::bloom_blur::bind_groups::BindGroup0,

    bloom_blur4: wgpu::TextureView,
    bloom_blur4_bindgroup: crate::shader::bloom_blur::bind_groups::BindGroup0,

    bloom_blur_combined: wgpu::TextureView,
    bloom_blur_combine_bindgroup: crate::shader::bloom_blur_combine::bind_groups::BindGroup0,
}

impl Textures {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let color = create_texture(device, width, height, "color texture", COLOR_FORMAT);
        let depth = create_texture(device, width, height, "depth texture", DEPTH_FORMAT);
        let bloom_bright = create_texture(
            device,
            width / 3,
            height / 3,
            "bloom bright texture",
            BLOOM_FORMAT,
        );
        let bloom_blur1 = create_texture(
            device,
            width / 6,
            height / 6,
            "bloom blur 1 texture",
            BLOOM_FORMAT,
        );
        let bloom_blur2 = create_texture(
            device,
            width / 12,
            height / 12,
            "bloom blur 2 texture",
            BLOOM_FORMAT,
        );
        let bloom_blur3 = create_texture(
            device,
            width / 24,
            height / 24,
            "bloom blur 3 texture",
            BLOOM_FORMAT,
        );
        let bloom_blur4 = create_texture(
            device,
            width / 48,
            height / 48,
            "bloom blur 4 texture",
            BLOOM_FORMAT,
        );
        let bloom_blur_combined = create_texture(
            device,
            width / 3,
            height / 3,
            "bloom blur combined texture",
            BLOOM_FORMAT,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let blit_bind_group = crate::shader::blit::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::blit::bind_groups::BindGroupLayout0 {
                color: &color,
                color_sampler: &sampler,
            },
        );

        let bloom_blur1_bindgroup =
            crate::shader::bloom_blur::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::bloom_blur::bind_groups::BindGroupLayout0 {
                    color: &bloom_bright,
                    color_sampler: &sampler,
                },
            );
        let bloom_blur2_bindgroup =
            crate::shader::bloom_blur::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::bloom_blur::bind_groups::BindGroupLayout0 {
                    color: &bloom_blur1,
                    color_sampler: &sampler,
                },
            );
        let bloom_blur3_bindgroup =
            crate::shader::bloom_blur::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::bloom_blur::bind_groups::BindGroupLayout0 {
                    color: &bloom_blur2,
                    color_sampler: &sampler,
                },
            );
        let bloom_blur4_bindgroup =
            crate::shader::bloom_blur::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::bloom_blur::bind_groups::BindGroupLayout0 {
                    color: &bloom_blur3,
                    color_sampler: &sampler,
                },
            );

        let bloom_blur_combine_bindgroup =
            crate::shader::bloom_blur_combine::bind_groups::BindGroup0::from_bindings(
                device,
                crate::shader::bloom_blur_combine::bind_groups::BindGroupLayout0 {
                    color1: &bloom_blur1,
                    color2: &bloom_blur2,
                    color3: &bloom_blur3,
                    color4: &bloom_blur4,
                    color_sampler: &sampler,
                },
            );

        let bloom_add_bindgroup = crate::shader::bloom_add::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::bloom_add::bind_groups::BindGroupLayout0 {
                color: &bloom_blur_combined,
                color_sampler: &sampler,
            },
        );

        Self {
            color,
            depth,
            blit_bind_group,
            bloom_bright,
            bloom_blur1,
            bloom_blur2,
            bloom_blur3,
            bloom_blur4,
            bloom_blur_combined,
            bloom_blur1_bindgroup,
            bloom_blur2_bindgroup,
            bloom_blur3_bindgroup,
            bloom_blur4_bindgroup,
            bloom_blur_combine_bindgroup,
            bloom_add_bindgroup,
        }
    }
}

fn create_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    texture.create_view(&Default::default())
}
