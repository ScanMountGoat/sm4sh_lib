use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use sm4sh_model::{AlphaFunc, DstFactor, NudMesh, SrcFactor};

use crate::{
    SharedData,
    renderer::{COLOR_FORMAT, DEPTH_FORMAT},
    shadergen::ShaderWgsl,
};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct ShaderKey {
    pub id: u32,
    pub alpha_test_ref: u16,
    pub alpha_func: AlphaFunc,
}

static SHADERS: LazyLock<Mutex<HashMap<Option<ShaderKey>, wgpu::ShaderModule>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn model_pipeline(
    device: &wgpu::Device,
    shared_data: &SharedData,
    mesh: &NudMesh,
) -> wgpu::RenderPipeline {
    let topology = match mesh.primitive_type {
        sm4sh_model::PrimitiveType::TriangleList => wgpu::PrimitiveTopology::TriangleList,
        sm4sh_model::PrimitiveType::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    };

    // TODO: Add unit tests for returning state structs.
    let blend = mesh.material1.as_ref().map(blend_state);

    let strip_index_format = topology.is_strip().then_some(wgpu::IndexFormat::Uint16);

    let cull_mode = mesh.material1.as_ref().and_then(|m| match m.cull_mode {
        sm4sh_model::CullMode::Disabled => None,
        sm4sh_model::CullMode::Outside => Some(wgpu::Face::Front),
        sm4sh_model::CullMode::Inside => Some(wgpu::Face::Back),
        sm4sh_model::CullMode::Disabled2 => None,
        sm4sh_model::CullMode::Inside2 => Some(wgpu::Face::Front),
        sm4sh_model::CullMode::Outside2 => Some(wgpu::Face::Back),
    });

    // TODO: Generate code for other materials as well?
    let key = mesh.material1.as_ref().map(|m| ShaderKey {
        id: m.shader_id,
        alpha_test_ref: m.alpha_test_ref,
        alpha_func: m.alpha_func,
    });

    // Shader IDs are often used more than once for expression meshes or split meshes.
    // Only compile unique shaders once to greatly reduce loading times.
    let mut shaders = SHADERS.lock().unwrap();
    let module = shaders
        .entry(key)
        .or_insert_with(|| {
            let program = key.and_then(|key| shared_data.database.get_shader(key.id));
            let alpha_test_ref_func = key.as_ref().map(|m| (m.alpha_test_ref, m.alpha_func));

            let shader_wgsl = ShaderWgsl::new(program.as_ref(), alpha_test_ref_func);
            let source = shader_wgsl.create_model_shader();
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(source)),
            })
        })
        .clone();
    drop(shaders);

    let label = key.map(|key| format!("{:X}", key.id));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: label.as_deref(),
        layout: Some(&shared_data.model_layout),
        vertex: crate::shader::model::vertex_state(
            &module,
            &crate::shader::model::vs_main_entry(wgpu::VertexStepMode::Vertex),
        ),
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format,
            cull_mode,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(crate::shader::model::fragment_state(
            &module,
            &crate::shader::model::fs_main_entry([Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend,
                write_mask: wgpu::ColorWrites::all(),
            })]),
        )),
        multiview: None,
        cache: None,
    })
}

fn blend_state(m: &sm4sh_model::NudMaterial) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: match m.src_factor {
                SrcFactor::One => wgpu::BlendFactor::One,
                SrcFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::One2 => wgpu::BlendFactor::One,
                SrcFactor::SourceAlpha2 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::Zero => wgpu::BlendFactor::Zero,
                SrcFactor::SourceAlpha3 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
                SrcFactor::DestinationAlpha7 => wgpu::BlendFactor::DstAlpha,
                SrcFactor::DestinationColor => wgpu::BlendFactor::Dst,
                SrcFactor::SrcAlpha3 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::SrcAlpha4 => wgpu::BlendFactor::SrcAlpha,
                SrcFactor::Unk16 => wgpu::BlendFactor::One,
                SrcFactor::Unk33 => wgpu::BlendFactor::One,
                SrcFactor::SrcAlpha5 => wgpu::BlendFactor::SrcAlpha,
            },
            dst_factor: match m.dst_factor {
                DstFactor::Zero => wgpu::BlendFactor::Zero,
                DstFactor::OneMinusSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One => wgpu::BlendFactor::One,
                DstFactor::OneReverseSubtract => wgpu::BlendFactor::One,
                DstFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
                DstFactor::SourceAlphaReverseSubtract => wgpu::BlendFactor::SrcAlpha,
                DstFactor::OneMinusDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
                DstFactor::One2 => wgpu::BlendFactor::One,
                DstFactor::Zero2 => wgpu::BlendFactor::Zero,
                DstFactor::Unk10 => wgpu::BlendFactor::Zero,
                DstFactor::OneMinusSourceAlpha2 => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One3 => wgpu::BlendFactor::One,
                DstFactor::Zero5 => wgpu::BlendFactor::Zero,
                DstFactor::Zero3 => wgpu::BlendFactor::Zero,
                DstFactor::One4 => wgpu::BlendFactor::One,
                DstFactor::OneMinusSourceAlpha3 => wgpu::BlendFactor::OneMinusSrcAlpha,
                DstFactor::One5 => wgpu::BlendFactor::One,
            },
            operation: match m.dst_factor {
                DstFactor::Zero => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha => wgpu::BlendOperation::Add,
                DstFactor::One => wgpu::BlendOperation::Add,
                DstFactor::OneReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
                DstFactor::SourceAlpha => wgpu::BlendOperation::Add,
                DstFactor::SourceAlphaReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
                DstFactor::OneMinusDestinationAlpha => wgpu::BlendOperation::Add,
                DstFactor::One2 => wgpu::BlendOperation::Add,
                DstFactor::Zero2 => wgpu::BlendOperation::Add,
                DstFactor::Unk10 => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha2 => wgpu::BlendOperation::Add,
                DstFactor::One3 => wgpu::BlendOperation::Add,
                DstFactor::Zero5 => wgpu::BlendOperation::Add,
                DstFactor::Zero3 => wgpu::BlendOperation::Add,
                DstFactor::One4 => wgpu::BlendOperation::Add,
                DstFactor::OneMinusSourceAlpha3 => wgpu::BlendOperation::Add,
                DstFactor::One5 => wgpu::BlendOperation::Add,
            },
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}
