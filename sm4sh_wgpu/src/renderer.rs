use glam::{vec4, Mat4, UVec4, Vec4};

use crate::{skeleton::BoneRenderer, CameraData, DeviceBufferExt, Model, QueueBufferExt};

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct Renderer {
    camera_buffer: wgpu::Buffer,
    model_bind_group0: crate::shader::model::bind_groups::BindGroup0,
    textures: Textures,
    bone_renderer: BoneRenderer,
    fb0_buffer: wgpu::Buffer,
    fb1_buffer: wgpu::Buffer,
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
                hdrRange: vec4(0.5, 2.0, 0.0, 0.0),
                colrHdrRange: Vec4::ZERO,
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

        let model_bind_group0 = crate::shader::model::bind_groups::BindGroup0::from_bindings(
            device,
            crate::shader::model::bind_groups::BindGroupLayout0 {
                camera: camera_buffer.as_entire_buffer_binding(),
                fb0: fb0_buffer.as_entire_buffer_binding(),
                fb1: fb1_buffer.as_entire_buffer_binding(),
                fb3: fb3_buffer.as_entire_buffer_binding(),
                fb4: fb4_buffer.as_entire_buffer_binding(),
                fb5: fb5_buffer.as_entire_buffer_binding(),
            },
        );

        let textures = Textures::new(device, width, height);

        let bone_renderer = BoneRenderer::new(device, &camera_buffer, output_format);

        Self {
            camera_buffer,
            model_bind_group0,
            textures,
            bone_renderer,
            fb0_buffer,
            fb1_buffer,
        }
    }

    pub fn render_model(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        model: &Model,
        camera: &CameraData,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Model Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
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

        self.model_bind_group0.set(&mut render_pass);
        model.draw(&mut render_pass, camera);

        self.bone_renderer
            .draw_bones(&mut render_pass, &model.bone_transforms, model.bone_count);
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

fn fb0(width: f32, height: f32) -> crate::shader::model::Fb0 {
    crate::shader::model::Fb0 {
        depthOfField0: vec4(0.0, 0.0, 0.0, 0.0),
        depthOfField1: vec4(0.0, 0.0, 0.0, 0.0),
        depthOfFieldTexSize: vec4(0.0, 0.0, 0.0, 0.0),
        projInvMatrix: Mat4::IDENTITY, // TODO: Fill in this value
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
        renderTargetTexSize: vec4(1.0 / width, 1.0 / height, 2.0 / width, 2.0 / height),
        glare_fog_param: [vec4(0.0, 0.0, 0.0, 0.0), vec4(0.0, 0.0, 0.0, 0.0)],
        glare_simple_color: vec4(0.0, 0.0, 0.0, 0.0),
        pad0_FB0: vec4(0.0, 0.0, 0.0, 0.0),
        lens_flare_param: vec4(0.0, 0.0, 0.0, 0.0),
        outline_param: vec4(0.25, 0.00, 0.00, 0.00),
        post_reflection_color: vec4(0.50, 0.50, 0.50, 0.20),
        MultiShadowMatrix: [Mat4::IDENTITY; 4], // TODO: fill in these values
        ShadowMapMatrix: Mat4::from_cols_array_2d(&[
            [0.00814, 0.00, 0.00, 0.00],
            [0.00, -0.00504, -0.01631, 0.00],
            [0.00, 0.01385, -0.00594, 0.00],
            [0.49189, 0.67917, 1.09728, 1.00],
        ]), // TODO: fill in these values
        view: Mat4::ZERO,                       // TODO: fill in these values
        eye: vec4(40.0, 47.40689, 37.02085, 1.0), // TODO: fill in these values
        constantColor: vec4(1.0, 1.0, 1.0, 1.0),
        lightMapPos: vec4(0.0, 0.0, 0.0, 0.0),
        reflectionGain: vec4(1.0, 1.0, 1.0, 1.0),
        hdrConstant: vec4(0.5, 2.0, 1.0, 1.0),
        _g_fresnelColor: vec4(1.0, 1.0, 1.0, 1.0),
        effect_light_param0: vec4(0.1, 0.1, -15.0, 0.0),
        effect_light_param1: vec4(30.0, 12.0, 29.0, 11.0),
        bgRotInv: Mat4::IDENTITY,
        reflectionColor1: vec4(0.0, 0.0, 0.0, 0.0),
        reflectionColor2: vec4(0.0001, 0.0, 0.0, 0.0),
        reflectionColor3: vec4(0.315, 0.31792, 0.35, 1.0),
        effect_light_param2: vec4(0.685, 0.68208, 0.65, 1.00),
    }
}

fn fb1() -> crate::shader::model::Fb1 {
    crate::shader::model::Fb1 {
        lightMapMatrix: Mat4::IDENTITY,
        blinkColor: vec4(1.0, 1.0, 1.0, 0.0),
        g_constantVolume: vec4(1.0, 1.0, 1.0, 1.0),
        g_constantOffset: vec4(0.0, 0.0, 0.0, 0.0),
        uvScrollCounter: vec4(0.35, 0.0, 0.0, 0.0), // TODO: changes over time?
        spycloakParams: vec4(-100.0, 0.0, 0.0, 0.0),
        compressParam: vec4(1.0, 0.0, 0.0, 0.0),
        g_fresnelColor: vec4(1.0, 1.0, 1.0, 1.0),
        depthOffset: vec4(0.0, 0.0, 0.0, 0.0),
        outlineColor: vec4(0.0, 0.0, 0.0, 1.0),
        pad0_FB1: [
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
            vec4(0.0, 0.0, 0.0, 0.0),
        ],
        lightMapColorGain: vec4(0.4875, 0.4875, 0.4875, 0.0),
        lightMapColorOffset: vec4(0.0, 0.0, 0.0, 0.0),
        ceilingDir: vec4(0.0, 1.0, 0.0, 0.0),
        ceilingColor: vec4(0.15, 0.15, 0.15, 0.0),
        groundColor: vec4(1.0, 1.0, 1.0, 0.0),
        ambientColor: vec4(0.0, 0.0, 0.0, 0.0),
        lightDirColor1: vec4(0.75, 0.75, 0.75, 0.0),
        lightDirColor2: vec4(0.2, 0.2, 0.2, 1.0),
        lightDirColor3: vec4(0.0, 0.0, 0.0, 0.0),
        lightDir1: vec4(0.0, -0.84323, -0.53756, 0.0),
        lightDir2: vec4(-0.87287, 0.43644, -0.21822, 0.0),
        lightDir3: vec4(0.0, 0.0, 0.0, 0.0),
        fogColor: vec4(1.0, 1.0, 1.0, 1.0),
        g_fresnelOffset: vec4(0.0, 0.0, 0.0, 0.0),
        ShadowMapParam: vec4(0.001, 0.0, 0.0, 0.0),
        charShadowColor: vec4(0.315, 0.31792, 0.35, 1.0),
        charShadowColor2: vec4(0.685, 0.68208, 0.65, 1.0),
        softLightingParams2: vec4(0.0, 0.0, 0.0, 1.0),
        bgShadowColor: vec4(0.81, 0.8175, 0.90, 1.0),
        g_iblColorGain: vec4(1.0, 1.0, 1.0, 0.0),
        g_iblColorOffset: vec4(0.15, 0.15, 0.15, 0.0),
        g_constantMin: Vec4::ZERO,
        loupeShadowParams: Vec4::ZERO,
        softLightColorGain: Vec4::ZERO,
        softLightColorOffset: Vec4::ZERO,
        characterColor: Vec4::ZERO,
    }
}

// Group resizable resources to avoid duplicating this logic.
pub struct Textures {
    depth: wgpu::TextureView,
}

impl Textures {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let depth = create_texture(device, width, height, "depth_texture", DEPTH_FORMAT);

        Self { depth }
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
