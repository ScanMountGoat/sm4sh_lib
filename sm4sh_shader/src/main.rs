use clap::{Parser, Subcommand};
use sm4sh_lib::nsh::{Gx2Shader, Nsh};
use sm4sh_model::shader_database::{ShaderDatabase, ShaderProgram};
use std::{collections::BTreeMap, fmt::Write, fs::File, path::Path};

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
    ShaderDatabase {
        /// Path to a text file with the output of the match-shaders command.
        shader_ids_shaders: String,
        /// The folder containing nsh shader binaries from a shader file like texas_cross.nsh
        nsh_shader_dump: String,
        /// Path for the output JSON database.
        output: String,
    },
}

fn main() -> anyhow::Result<()> {
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
        Commands::ShaderDatabase {
            shader_ids_shaders,
            nsh_shader_dump,

            output,
        } => create_shader_database(&shader_ids_shaders, &nsh_shader_dump, &output)?,
    }
    println!("Finished in {:?}", start.elapsed());
    Ok(())
}

fn dump_shaders(nsh: &str, output: &str, gfd_tool: &str) -> anyhow::Result<()> {
    let nsh_path = Path::new(&nsh);
    let nsh = Nsh::from_file(nsh_path)?;

    let output = Path::new(output);
    std::fs::create_dir_all(output).unwrap();

    let name = nsh_path.file_stem().unwrap().to_string_lossy().to_string();

    for (i, shader) in nsh.shaders.iter().enumerate() {
        let gx2 = shader.gfx2.gx2_shader()?;
        let gx2_path = output.join(&name).with_extension(format!("{i}.gx2.bin"));
        gx2.save(gx2_path)?;

        let binary_path = output.join(&name).with_extension(format!("{i}.bin"));
        std::fs::write(&binary_path, gx2.program_binary())?;

        let txt_path = output.join(&name).with_extension(format!("{i}.txt"));
        dissassemble_shader(&binary_path, &txt_path, gfd_tool);
    }
    Ok(())
}

fn dissassemble_shader(binary_path: &Path, txt_path: &Path, gfd_tool: &str) {
    std::process::Command::new(gfd_tool)
        .arg("disassemble")
        .arg(binary_path)
        .stdout(File::create(txt_path).unwrap())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
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
    for (name, shader_id) in std::fs::read_to_string(shader_names)
        .unwrap()
        .lines()
        .zip(ids)
    {
        let name = name.trim().strip_prefix("shader_").unwrap();

        // TODO: why does this not match vertex shaders?
        for tag in ["_vs", "_ps"] {
            let path = Path::new(cemu_shader_dump).join(format!("{name}{tag}.bin"));
            if let Ok(cemu_bytes) = std::fs::read(path) {
                for (sm4sh_path, sm4sh_bytes) in &sm4sh_shaders {
                    if sm4sh_bytes == &cemu_bytes {
                        let sm4sh_name = sm4sh_path.file_stem().unwrap().to_string_lossy();
                        writeln!(&mut text, "{shader_id:X?}, {name}, {sm4sh_name}")?;
                        break;
                    }
                }
            }
        }
    }
    std::fs::write(output, text)?;
    Ok(())
}

fn create_shader_database(
    shader_ids_shaders: &str,
    nsh_shader_dump: &str,
    output: &str,
) -> anyhow::Result<()> {
    let mut programs = BTreeMap::new();
    for line in std::fs::read_to_string(shader_ids_shaders).unwrap().lines() {
        let parts: Vec<_> = line.split(",").map(|s| s.trim()).collect();
        let shader_id = parts[0].to_string();
        let nsh_index: usize = parts[2].strip_prefix("texas_cross.").unwrap().parse()?;

        let gx2_path = Path::new(nsh_shader_dump).join(format!("texas_cross.{nsh_index}.gx2.bin"));
        let gx2 = Gx2Shader::from_file(gx2_path)?;

        let gx2_samplers = match &gx2 {
            Gx2Shader::Vertex(v) => &v.sampler_vars,
            Gx2Shader::Pixel(p) => &p.sampler_vars,
        };
        let samplers = gx2_samplers
            .iter()
            .map(|s| (s.location as usize, s.name.clone()))
            .collect();

        // NU_ parameters are in the MC block.
        let mut parameters = BTreeMap::new();
        if let Some(block_index) = gx2.uniform_blocks().iter().position(|b| b.name == "MC") {
            for var in gx2.uniform_vars().iter() {
                if var.uniform_buffer_index == block_index as i32 {
                    parameters.insert(var.offset as usize, var.name.clone());
                }
            }
        }

        programs.insert(
            shader_id,
            ShaderProgram {
                samplers,
                parameters,
            },
        );
    }

    let database = ShaderDatabase { programs };
    let json = serde_json::to_string_pretty(&database)?;
    std::fs::write(output, &json)?;
    Ok(())
}
