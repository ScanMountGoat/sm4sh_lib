use std::{collections::BTreeSet, fmt::Write, path::Path};

use log::error;
use sm4sh_lib::gx2::{Gx2PixelShader, Gx2VertexShader};
use smol_str::SmolStr;
use xc3_shader::graph::{Expr, Graph};

pub fn annotate_shader(vert_asm_path: &Path) -> anyhow::Result<()> {
    let name = vert_asm_path.with_extension("");
    let name = name.file_stem().unwrap().to_string_lossy();

    let vert_asm = std::fs::read_to_string(vert_asm_path)?;

    let frag_asm_path = vert_asm_path.with_file_name(format!("{name}.frag.txt"));
    let frag_asm = std::fs::read_to_string(frag_asm_path)?;

    let vert_gx2_path = vert_asm_path.with_file_name(format!("{name}.vert.gx2.bin"));
    let vert = Gx2VertexShader::from_file(vert_gx2_path)?;

    let frag_gx2_path = vert_asm_path.with_file_name(format!("{name}.frag.gx2.bin"));
    let frag = Gx2PixelShader::from_file(frag_gx2_path)?;

    let vertex_glsl = annotate_vertex_shader(&vert_asm, &vert)?;
    let frag_glsl = annotate_fragment_shader(&frag_asm, &vert, &frag)?;

    std::fs::write(
        vert_asm_path.with_file_name(format!("{name}.vert")),
        &vertex_glsl,
    )?;
    std::fs::write(
        vert_asm_path.with_file_name(format!("{name}.frag")),
        &frag_glsl,
    )?;

    Ok(())
}

// TODO: Share annotation code with xc3_shader.
fn annotate_vertex_shader(
    latte_asm: &str,
    shader: &Gx2VertexShader,
) -> Result<String, anyhow::Error> {
    let mut graph = Graph::from_latte_asm(latte_asm)?;

    for i in 0..graph.exprs.len() {
        replace_uniform(i, &mut graph, &shader.uniform_blocks, &shader.uniform_vars);
    }

    let mut outputs = BTreeSet::new();
    for node in &mut graph.nodes {
        for prefix in ["PIX", "PARAM"] {
            if node.output.name.starts_with(prefix) {
                let index: usize = node.output.name.trim_start_matches(prefix).parse()?;
                outputs.insert(index);

                node.output.name = format!("out_attr{index}").into();
            }
        }
    }
    let glsl = graph.to_glsl();
    let mut annotated = String::new();
    write_uniform_blocks(&mut annotated, &shader.uniform_blocks, &shader.uniform_vars)?;
    for attribute in &shader.attributes {
        writeln!(
            &mut annotated,
            "layout(location = {}) in vec4 {};",
            attribute.location, attribute.name
        )?;
    }

    writeln!(&mut annotated)?;

    let output_count = shader
        .registers
        .spi_vs_out_id
        .iter()
        .flat_map(|id| id.to_be_bytes())
        .filter(|i| *i != 0xFF)
        .count();

    for i in 0..output_count {
        writeln!(
            &mut annotated,
            "layout(location = {i}) out vec4 out_attr{i};"
        )?;
    }

    writeln!(&mut annotated)?;
    writeln!(&mut annotated, "void main() {{")?;

    // Vertex input attribute registers can also be remapped.
    for (i, location) in shader
        .registers
        .sq_vtx_semantic
        .iter()
        .enumerate()
        .take(shader.registers.num_sq_vtx_semantic as usize)
    {
        if *location != 0xFF {
            if let Some(a) = shader.attributes.iter().find(|a| a.location == *location) {
                // Register 0 is special, so we need to start with register 1.
                for c in "xyzw".chars() {
                    writeln!(&mut annotated, "    R{}.{c} = {}.{c};", i + 1, a.name).unwrap();
                }
            } else {
                error!("Unable to find name for attribute location {location}");
            }
        }
    }

    for line in glsl.lines() {
        writeln!(&mut annotated, "    {line}")?;
    }
    writeln!(&mut annotated, "}}")?;
    Ok(annotated)
}

fn annotate_fragment_shader(
    latte_asm: &str,
    vertex_shader: &Gx2VertexShader,
    frag_shader: &Gx2PixelShader,
) -> Result<String, anyhow::Error> {
    let mut graph = Graph::from_latte_asm(latte_asm)?;

    let mut texture_names = Vec::new();
    for e in &graph.exprs {
        if let Expr::Func { name, args, .. } = e {
            if name.starts_with("texture") {
                if let Some(Expr::Global { name, .. }) = args.first().map(|a| &graph.exprs[*a]) {
                    texture_names.push(name.clone());
                }
            }
        }
    }

    for i in 0..graph.exprs.len() {
        if let Expr::Global { name, .. } = &mut graph.exprs[i] {
            // The name of the texture is its binding location.
            // texture(t15, ...) -> texture(g_VSMTextureSampler, ...)
            if texture_names.contains(&name) {
                if let Some(index) = name.strip_prefix("t").and_then(|n| n.parse::<usize>().ok()) {
                    if let Some(sampler_name) = frag_shader.sampler_vars.iter().find_map(|s| {
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

        replace_uniform(
            i,
            &mut graph,
            &frag_shader.uniform_blocks,
            &frag_shader.uniform_vars,
        )
    }

    let mut outputs = BTreeSet::new();
    for node in &mut graph.nodes {
        for prefix in ["PIX", "PARAM"] {
            if node.output.name.starts_with(prefix) {
                let index: usize = node.output.name.trim_start_matches(prefix).parse()?;
                outputs.insert(index);

                node.output.name = format!("out_attr{index}").into();
            }
        }
    }
    let glsl = graph.to_glsl();
    let mut annotated = String::new();
    write_uniform_blocks(
        &mut annotated,
        &frag_shader.uniform_blocks,
        &frag_shader.uniform_vars,
    )?;
    for sampler in &frag_shader.sampler_vars {
        writeln!(
            &mut annotated,
            "layout(binding = {}) uniform {} {};",
            sampler.location,
            sampler_type(sampler),
            sampler.name
        )?;
    }
    writeln!(&mut annotated)?;

    let input_locations = fragment_input_locations(vertex_shader, frag_shader);

    for (i, location) in input_locations.iter().enumerate() {
        writeln!(
            &mut annotated,
            "layout(location = {location}) in vec4 in_attr{i};"
        )?;
    }
    writeln!(&mut annotated)?;

    for i in outputs {
        writeln!(
            &mut annotated,
            "layout(location = {i}) out vec4 out_attr{i};"
        )?;
    }

    writeln!(&mut annotated)?;
    writeln!(&mut annotated, "void main() {{")?;

    // Fragment input attributes initialize R0, R1, ...?
    for i in 0..input_locations.len() {
        for c in "xyzw".chars() {
            writeln!(&mut annotated, "    R{i}.{c} = in_attr{i}.{c};").unwrap();
        }
    }

    for line in glsl.lines() {
        writeln!(&mut annotated, "    {line}")?;
    }
    writeln!(&mut annotated, "}}")?;
    Ok(annotated)
}

fn fragment_input_locations(
    vertex_shader: &Gx2VertexShader,
    frag_shader: &Gx2PixelShader,
) -> Vec<i32> {
    // Fragment inputs are remapped by vertex and fragment registers.
    // https://github.com/decaf-emu/decaf-emu/blob/e6c528a20a41c34e0f9eb91dd3da40f119db2dee/src/libgpu/src/spirv/spirv_transpiler.cpp#L280-L301
    let mut input_locations = Vec::new();

    for input_id in frag_shader
        .registers
        .spi_ps_input_cntls
        .iter()
        .take(frag_shader.registers.num_spi_ps_input_cntl as usize)
    {
        let mut i = 0;
        for register in &vertex_shader.registers.spi_vs_out_id {
            // The order is [id3, id2, id1, id0];
            for id in &register.to_le_bytes() {
                if *id == (input_id & 0xFF) as u8 {
                    input_locations.push(i);
                }

                i += 1;
            }
        }
    }

    input_locations
}

fn replace_uniform(
    expr_index: usize,
    graph: &mut Graph,
    blocks: &[sm4sh_lib::gx2::UniformBlock],
    vars: &[sm4sh_lib::gx2::UniformVar],
) {
    let result = uniform_block_name_var_name(expr_index, &graph, blocks, vars);
    if let Expr::Parameter {
        name, field, index, ..
    } = &mut graph.exprs[expr_index]
    {
        if let Some((new_name, new_field)) = result {
            *name = new_name;
            *field = Some(new_field);
            *index = None;
        }
    }
}

fn uniform_block_name_var_name(
    expr_index: usize,
    graph: &Graph,
    blocks: &[sm4sh_lib::gx2::UniformBlock],
    vars: &[sm4sh_lib::gx2::UniformVar],
) -> Option<(SmolStr, SmolStr)> {
    if let Expr::Parameter { name, index, .. } = &graph.exprs[expr_index] {
        if let Some(constant_buffer_index) = name
            .strip_prefix("CB")
            .and_then(|i| i.parse::<usize>().ok())
        {
            if let Some(block_index) = blocks
                .iter()
                .position(|b| b.offset as usize == constant_buffer_index)
            {
                let block = &blocks[block_index];

                // TODO: Don't assume vec4 for all uniforms when converting indices to offsets.
                // TODO: Are indices in terms of floats?
                // TODO: group uniforms into blocks to make this easier.
                let i = index.and_then(|i| graph.exprs.get(i));
                if let Some(var) = vars.iter().find(|v| {
                    v.uniform_block_index == block_index as i32
                        && matches!(i.as_deref(), Some(Expr::Int(i)) if *i * 4 == v.offset as i32)
                }) {
                    return Some(((&block.name).into(), var.name.clone().into()));
                }
            }
        }
    }

    None
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
            .filter(|v| v.uniform_block_index == i as i32)
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

fn sampler_type(sampler: &sm4sh_lib::gx2::SamplerVar) -> &'static str {
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
