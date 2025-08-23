use std::{collections::BTreeSet, fmt::Write, path::Path};

use sm4sh_lib::nsh::Gx2Shader;
use xc3_shader::graph::{Expr, Graph};

pub fn annotate_shader(txt_path: &Path) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(txt_path)?;

    let name = txt_path.file_stem().unwrap().to_string_lossy();
    let gx2_path = txt_path.with_file_name(format!("{name}.gx2.bin"));
    let shader = Gx2Shader::from_file(gx2_path)?;

    let annotated = annotate_gx2_shader(&text, &shader)?;

    std::fs::write(txt_path.with_file_name(format!("{name}.glsl")), &annotated)?;
    Ok(())
}

// TODO: Share annotation code with xc3_shader.
fn annotate_gx2_shader(latte_asm: &str, shader: &Gx2Shader) -> Result<String, anyhow::Error> {
    let mut graph = Graph::from_latte_asm(latte_asm)?;

    for node in &mut graph.nodes {
        if let Gx2Shader::Pixel(pixel) = &shader {
            if let Expr::Func { name, args, .. } = &mut node.input {
                if name.starts_with("texture")
                    && let Some(Expr::Global { name, .. }) = args.first_mut()
                {
                    // The name of the texture is its binding location.
                    // texture(t15, ...) -> texture(g_VSMTextureSampler, ...)
                    if let Some(index) =
                        name.strip_prefix("t").and_then(|n| n.parse::<usize>().ok())
                    {
                        if let Some(sampler_name) = pixel.sampler_vars.iter().find_map(|s| {
                            if s.location as usize == index {
                                Some(&s.name)
                            } else {
                                None
                            }
                        }) {
                            *name = sampler_name.into();
                        }
                    }
                }
            }
        }

        node.input
            .visit_exprs_mut(&mut |e| replace_uniform(e, shader.uniform_blocks()));
    }
    let mut output_locations = BTreeSet::new();
    for node in &mut graph.nodes {
        for prefix in ["PIX", "PARAM"] {
            if node.output.name.starts_with(prefix) {
                let index: usize = node.output.name.trim_start_matches(prefix).parse()?;
                output_locations.insert(index);

                node.output.name = format!("out_attr{index}").into();
            }
        }
    }
    let glsl = graph.to_glsl();
    let mut annotated = String::new();
    write_uniform_blocks(
        &mut annotated,
        shader.uniform_blocks(),
        shader.uniform_vars(),
    )?;
    if let Gx2Shader::Pixel(pixel) = &shader {
        for sampler in &pixel.sampler_vars {
            writeln!(
                &mut annotated,
                "layout(binding = {}) uniform {} {};",
                sampler.location,
                sampler_type(sampler),
                sampler.name
            )?;
        }
    }
    if let Gx2Shader::Vertex(vertex) = &shader {
        for attribute in &vertex.attributes {
            writeln!(
                &mut annotated,
                "layout(location = {}) in vec4 {};",
                attribute.location, attribute.name
            )?;
        }
    }
    writeln!(&mut annotated)?;
    for i in output_locations.iter() {
        writeln!(
            &mut annotated,
            "layout(location = {i}) out vec4 out_attr{i};"
        )?;
    }
    writeln!(&mut annotated)?;
    writeln!(&mut annotated, "void main() {{")?;
    for line in glsl.lines() {
        writeln!(&mut annotated, "    {line}")?;
    }
    writeln!(&mut annotated, "}}")?;
    Ok(annotated)
}

fn replace_uniform(e: &mut Expr, blocks: &[sm4sh_lib::gx2::UniformBlock]) {
    if let Expr::Parameter { name, field, .. } = e {
        match name.as_str() {
            "KC0" => {
                // TODO: What is the correct way to map KC0 to uniform blocks?
                if let Some(block) = blocks.iter().find(|b| b.offset == 1) {
                    *field = Some("values".into());
                    *name = (&block.name).into();
                }
            }
            "KC1" => {
                // TODO: What is the correct way to map KC1 to uniform blocks?
                if let Some(block) = blocks.iter().find(|b| b.offset == 2) {
                    *field = Some("values".into());
                    *name = (&block.name).into();
                }
            }
            _ => (),
        }
    }
}

fn write_uniform_blocks(
    annotated: &mut String,
    blocks: &[sm4sh_lib::gx2::UniformBlock],
    vars: &[sm4sh_lib::gx2::UniformVar],
) -> anyhow::Result<()> {
    for (i, block) in blocks.iter().enumerate() {
        writeln!(
            annotated,
            "layout(binding = {}, std140) uniform _{} {{",
            block.offset, &block.name
        )?;

        let mut block_vars: Vec<_> = vars
            .iter()
            .filter(|v| v.uniform_buffer_index == i as i32)
            .collect();
        block_vars.sort_by_key(|v| v.offset);

        for var in block_vars {
            // TODO: will arrays always have a var representing the entire array?
            if !var.name.contains("[") {
                // TODO: Calculate the appropriate position based on offsets.
                let ty = data_type(var);
                if var.count > 1 {
                    writeln!(annotated, "    {ty} {}[{}];", var.name, var.count)?;
                } else {
                    writeln!(annotated, "    {ty} {};", var.name)?;
                }
            }
        }
        writeln!(annotated, "    vec4 values[{}];", block.size / 16)?;
        writeln!(annotated, "}} {};", &block.name)?;
        writeln!(annotated)?;
    }
    Ok(())
}

fn data_type(var: &sm4sh_lib::gx2::UniformVar) -> &'static str {
    match var.data_type {
        sm4sh_lib::gx2::VarType::Void => "void",
        sm4sh_lib::gx2::VarType::Bool => "bool",
        sm4sh_lib::gx2::VarType::Float => "float",
        sm4sh_lib::gx2::VarType::Vec2 => "vec2",
        sm4sh_lib::gx2::VarType::Vec3 => "vec3",
        sm4sh_lib::gx2::VarType::Vec4 => "vec4",
        sm4sh_lib::gx2::VarType::IVec2 => "ivec2",
        sm4sh_lib::gx2::VarType::IVec4 => "ivec4",
        sm4sh_lib::gx2::VarType::UVec4 => "uvec4",
        sm4sh_lib::gx2::VarType::Mat2x4 => "mat2x2",
        sm4sh_lib::gx2::VarType::Mat3x4 => "mat3x4",
        sm4sh_lib::gx2::VarType::Mat4 => "mat4",
    }
}

fn sampler_type(sampler: &sm4sh_lib::gx2::Sampler) -> &'static str {
    match sampler.sampler_type {
        sm4sh_lib::gx2::SamplerType::D1 => "sampler1D",
        sm4sh_lib::gx2::SamplerType::D2 => "sampler2D",
        sm4sh_lib::gx2::SamplerType::Unk2 => "",
        sm4sh_lib::gx2::SamplerType::Unk3 => "",
        sm4sh_lib::gx2::SamplerType::Cube => "samplerCube",
        sm4sh_lib::gx2::SamplerType::Unk10 => "",
        sm4sh_lib::gx2::SamplerType::Unk13 => "",
    }
}
