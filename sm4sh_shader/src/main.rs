use anyhow::Context;
use clap::{Parser, Subcommand};
use log::error;
use rayon::prelude::*;
use sm4sh_lib::{
    gx2::{Gx2PixelShader, Gx2VertexShader},
    nsh::Nsh,
};
use sm4sh_model::database::{ShaderDatabase, ShaderProgram};
use smol_str::SmolStr;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs::File,
    path::Path,
};
use xc3_shader::graph::glsl::{GlslGraph, glsl_dependencies};

use crate::{
    annotation::annotate_shader,
    database::{convert_expr, shader_from_glsl},
};

mod annotation;
mod database;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    DumpShaders {
        /// The input .nsh shader file.
        nsh: String,
        /// The output folder for the disassembled shaders.
        output_folder: String,
        /// The path to the gfd-tool executable
        gfd_tool: String,
    },
    /// Find the program in the nsh for each material shader ID value using shader dumps.
    MatchShaders {
        /// The path to a a text file with one ID like "92000161" per line.
        shader_ids: String,
        /// The path to a text file with one RenderDoc Cemu shader name like "shader_8e2dda0cc310098f_0000000000000079" per line.
        shader_names: String,
        /// The folder containing the output of the dump-shaders command.
        nsh_shader_dump: String,
        /// Cemu's dump/shaders folder to match with the nsh shaders and shader names.
        cemu_shader_dump: String,
        /// Path for the output txt file.
        output: String,
    },
    /// Convert shaders to annotated GLSL shaders.
    AnnotateShaders {
        /// The folder containing the output of the dump-shaders command.
        nsh_shader_dump: String,
    },
    /// Convert annotated GLSL shaders to a shader database.
    ShaderDatabase {
        /// The text file with the output of the match-shaders command.
        shader_ids_shaders: String,
        /// The folder containing the output of the annotate-shaders command.
        nsh_shader_dump: String,
        /// The output database.
        output: String,
    },
    /// Find output dependencies for the given GLSL shader program.
    GlslOutputDependencies {
        /// The input fragment GLSL file.
        frag: String,
        /// The output txt file.
        output: String,
    },
    /// Find all lines of GLSL code influencing the final assignment of a variable.
    GlslDependencies {
        /// The input GLSL file.
        input: String,
        /// The output GLSL file.
        output: String,
        /// The name of the variable to analyze.
        var: String,
    },
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()?;

    let cli = Cli::parse();
    let start = std::time::Instant::now();
    match cli.command {
        Commands::DumpShaders {
            nsh,
            output_folder,
            gfd_tool,
        } => dump_shaders(&nsh, &output_folder, &gfd_tool)?,
        Commands::MatchShaders {
            shader_ids,
            shader_names,
            nsh_shader_dump,
            cemu_shader_dump,
            output,
        } => match_shaders_to_nsh(
            &shader_ids,
            &shader_names,
            &nsh_shader_dump,
            &cemu_shader_dump,
            &output,
        )?,
        Commands::AnnotateShaders { nsh_shader_dump } => annotate_shaders(&nsh_shader_dump)?,
        Commands::ShaderDatabase {
            shader_ids_shaders,
            nsh_shader_dump,
            output,
        } => create_shader_database(&shader_ids_shaders, &nsh_shader_dump, &output)?,
        Commands::GlslOutputDependencies { frag, output } => {
            glsl_output_dependencies(&frag, &output)?
        }
        Commands::GlslDependencies { input, output, var } => {
            let source = std::fs::read_to_string(input).unwrap();
            let (var, channels) = var.split_once('.').unwrap_or((&var, ""));
            let source_out = glsl_dependencies(&source, var, channels.chars().next());
            std::fs::write(output, source_out).unwrap();
        }
    }
    println!("Finished in {:?}", start.elapsed());
    Ok(())
}

fn dump_shaders(nsh: &str, output: &str, gfd_tool: &str) -> anyhow::Result<()> {
    let nsh_path = Path::new(&nsh);
    let nsh = Nsh::from_file(nsh_path)?;

    let output = Path::new(output);
    std::fs::create_dir_all(output)?;

    let name = nsh_path.file_stem().unwrap().to_string_lossy().to_string();

    nsh.programs
        .par_iter()
        .enumerate()
        .try_for_each(|(i, program)| {
            // Extract vertex shader.
            let gx2 = program.vertex_gx2()?;
            let gx2_path = output.join(format!("{name}.{i}.vert.gx2.bin"));
            gx2.save(gx2_path)?;

            let binary_path = output.join(format!("{name}.{i}.vert.bin"));
            std::fs::write(&binary_path, &gx2.program_binary)?;

            let txt_path = output.join(format!("{name}.{i}.vert.txt"));
            dissassemble_shader(&binary_path, &txt_path, gfd_tool)?;

            // Extract pixel shader.
            let gx2 = program.pixel_gx2()?;
            let gx2_path = output.join(format!("{name}.{i}.frag.gx2.bin"));
            gx2.save(gx2_path)?;

            let binary_path = output.join(format!("{name}.{i}.frag.bin"));
            std::fs::write(&binary_path, &gx2.program_binary)?;

            let txt_path = output.join(format!("{name}.{i}.frag.txt"));
            dissassemble_shader(&binary_path, &txt_path, gfd_tool)?;
            Ok(())
        })
}

fn dissassemble_shader(binary_path: &Path, txt_path: &Path, gfd_tool: &str) -> anyhow::Result<()> {
    std::process::Command::new(gfd_tool)
        .arg("disassemble")
        .arg(binary_path)
        .stdout(File::create(txt_path)?)
        .spawn()?
        .wait()?;
    Ok(())
}

fn match_shaders_to_nsh(
    shader_ids: &str,
    shader_names: &str,
    nsh_shader_dump: &str,
    cemu_shader_dump: &str,
    output: &str,
) -> anyhow::Result<()> {
    let mut ids = Vec::new();
    for line in std::fs::read_to_string(shader_ids)?.lines() {
        ids.push(u32::from_str_radix(line, 16)?);
    }

    // Read nsh binaries only once.
    let mut sm4sh_shaders = Vec::new();
    for entry in std::fs::read_dir(nsh_shader_dump)? {
        let sm4sh_path = entry?.path();
        if sm4sh_path.extension().and_then(|e| e.to_str()) == Some("bin") {
            let sm4sh_bytes = std::fs::read(&sm4sh_path)?;
            sm4sh_shaders.push((sm4sh_path, sm4sh_bytes));
        }
    }

    // Each ID like 92000161 has a pixel shader name like "shader_8e2dda0cc310098f_0000000000000079" from Cemu in RenderDoc.
    // This matches a binary like 8e2dda0cc310098f_0000000000000079_ps.bin in the Cemu shader dump.
    // This compiled WiiU shader binary can then be used to find the shader index in texas_cross.nsh.
    // In practice, IDs in order starting from 92000161 have increasing indices.
    // The gap between indices varies, so this needs to be precomputed using shader dumps.
    let mut text = String::new();
    for (name, shader_id) in std::fs::read_to_string(shader_names)?.lines().zip(ids) {
        let names: Vec<_> = name
            .split(",")
            .map(|n| n.trim().strip_prefix("shader_").unwrap())
            .collect();

        for (name, tag) in names.iter().zip(["_vs", "_ps"]) {
            let path = Path::new(cemu_shader_dump).join(format!("{name}{tag}.bin"));
            let cemu_bytes = std::fs::read(path)?;

            if let Some(sm4sh_path) = sm4sh_shaders.iter().find_map(|(sm4sh_path, sm4sh_bytes)| {
                if sm4sh_bytes == &cemu_bytes {
                    Some(sm4sh_path)
                } else {
                    None
                }
            }) {
                let sm4sh_name = sm4sh_path.file_stem().unwrap().to_string_lossy();
                writeln!(&mut text, "{shader_id:08X?}, {name}, {sm4sh_name}")?;
            } else {
                error!("Unable to match {shader_id:08X} {name}{tag}");
            }
        }
    }
    std::fs::write(output, text)?;
    Ok(())
}

fn annotate_shaders(nsh_shader_dump: &str) -> anyhow::Result<()> {
    globwalk::GlobWalkerBuilder::from_patterns(nsh_shader_dump, &["*.vert.txt"])
        .build()?
        .filter_map(|e| e.ok())
        .par_bridge()
        .for_each(|entry| {
            let path = entry.path().to_path_buf();
            if let Err(e) =
                annotate_shader(&path).with_context(|| format!("failed to process {path:?}"))
            {
                println!("{e:?}");
            }
        });
    Ok(())
}

fn create_shader_database(
    shader_ids_shaders: &str,
    nsh_shader_dump: &str,
    output: &str,
) -> anyhow::Result<()> {
    let folder = Path::new(nsh_shader_dump);

    let programs = std::fs::read_to_string(shader_ids_shaders)?
        .lines()
        .par_bridge()
        .map(|line| {
            let parts: Vec<_> = line.split(",").map(|s| s.trim()).collect();
            let shader_id = u32::from_str_radix(parts[0], 16)?;
            let nsh_index: usize = parts[2].split(".").nth(1).unwrap().parse()?;

            let gx2_path = folder.join(format!("texas_cross.{nsh_index}.frag.gx2.bin"));
            let frag_gx2 = Gx2PixelShader::from_file(gx2_path)?;

            let gx2_path = folder.join(format!("texas_cross.{nsh_index}.vert.gx2.bin"));
            let vert_gx2 = Gx2VertexShader::from_file(gx2_path)?;

            // Sort string names in ascending order by their location.
            let samplers: BTreeMap<_, _> = frag_gx2
                .sampler_vars
                .iter()
                .map(|s| (s.location, SmolStr::from(&s.name)))
                .collect();

            // NU_ parameters are in the MC block.
            let mut parameters = BTreeMap::new();
            if let Some(block_index) = frag_gx2.uniform_blocks.iter().position(|b| b.name == "MC") {
                for var in frag_gx2.uniform_vars.iter() {
                    if var.uniform_block_index == block_index as i32 {
                        parameters.insert(var.offset, SmolStr::from(&var.name));
                    }
                }
            }

            let vert_path = folder.join(format!("texas_cross.{nsh_index}.vert"));
            let vertex = std::fs::read_to_string(vert_path)?;
            let vertex = GlslGraph::parse_glsl(&vertex)?;

            let frag_path = folder.join(format!("texas_cross.{nsh_index}.frag"));
            let fragment = std::fs::read_to_string(frag_path)?;
            let fragment = GlslGraph::parse_glsl(&fragment)?;

            let program = shader_from_glsl(vertex, fragment);

            let attributes: BTreeMap<_, _> = vert_gx2
                .attributes
                .iter()
                .map(|a| (a.location, SmolStr::from(&a.name)))
                .collect();

            Ok((
                shader_id,
                ShaderProgram {
                    output_dependencies: program.output_dependencies,
                    exprs: program.exprs.into_iter().map(convert_expr).collect(),
                    attributes: attributes.into_values().collect(),
                    samplers: samplers.into_values().collect(),
                    parameters: parameters.into_values().collect(),
                },
            ))
        })
        .collect::<anyhow::Result<_>>()?;

    let database = ShaderDatabase::from_programs(programs);
    database.save(output)?;
    Ok(())
}

fn glsl_output_dependencies(frag: &str, output: &str) -> anyhow::Result<()> {
    // TODO: make an argument for the vertex path?
    let vert_glsl = std::fs::read_to_string(Path::new(&frag).with_extension("vert"))?;
    let vert = GlslGraph::parse_glsl(&vert_glsl)?;

    let frag_glsl = std::fs::read_to_string(frag)?;
    let fragment = GlslGraph::parse_glsl(&frag_glsl)?;

    // TODO: graphviz support
    let shader = shader_from_glsl(vert, fragment);
    std::fs::write(output, shader_str(&shader)?)?;
    Ok(())
}

pub fn shader_str(s: &crate::database::ShaderProgram) -> anyhow::Result<String> {
    // Use a condensed representation similar to GLSL for nicer diffs.
    let mut output = String::new();
    for (k, v) in &s.output_dependencies {
        let mut visited = BTreeSet::new();
        write_expr_dependencies_recursive(&mut output, s, *v, &mut visited)?;
        writeln!(&mut output, "{k} = var{v};")?;
        writeln!(&mut output)?;
    }
    Ok(output)
}

fn write_expr_dependencies_recursive(
    output: &mut String,
    s: &crate::database::ShaderProgram,
    i: usize,
    visited: &mut BTreeSet<usize>,
) -> anyhow::Result<()> {
    // Write all values that this value depends on first.
    if visited.insert(i) {
        let expr = &s.exprs[i];
        match expr {
            xc3_shader::expr::OutputExpr::Value(xc3_shader::expr::Value::Texture(t)) => {
                for arg in t.texcoords.iter() {
                    write_expr_dependencies_recursive(output, s, *arg, visited)?;
                }
            }
            xc3_shader::expr::OutputExpr::Func { args, .. } => {
                for arg in args.iter() {
                    write_expr_dependencies_recursive(output, s, *arg, visited)?;
                }
            }
            xc3_shader::expr::OutputExpr::Value(_) => (),
        }
        writeln!(output, "var{i} = {expr};")?;
    }
    Ok(())
}
