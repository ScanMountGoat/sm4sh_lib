use clap::{Parser, Subcommand};
use sm4sh_model::shader_database::{ShaderDatabase, ShaderProgram};
use std::{collections::BTreeMap, path::Path};

mod gfx2;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Find the program in the nsh for each material flags value using shader dumps.
    MatchShaders {
        /// The path to a a text file with one flag like "92000161" per line.
        flags: String,
        /// The path to a text file with one RenderDoc Cemu shader name like "shader_8e2dda0cc310098f_0000000000000079" per line.
        shader_names: String,
        /// The folder containing nsh shader binaries from a shader file like texas_cross.nsh
        nsh_shader_dump: String,
        /// Cemu's dump/shaders folder to match with the nsh shaders and shader names.
        cemu_shader_dump: String,
    },
    ShaderDatabase {
        /// Path to a text file with the output of the match-shaders command.
        flags_shaders: String,
        /// The folder containing nsh shader binaries from a shader file like texas_cross.nsh
        nsh_shader_dump: String,
        /// Path for the output JSON database.
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::MatchShaders {
            flags,
            shader_names,
            nsh_shader_dump,
            cemu_shader_dump,
        } => {
            map_shaders_to_nsh(&flags, &shader_names, &nsh_shader_dump, &cemu_shader_dump);
        }
        Commands::ShaderDatabase {
            flags_shaders,
            nsh_shader_dump,

            output,
        } => create_shader_database(&flags_shaders, &nsh_shader_dump, &output),
    }
}

fn map_shaders_to_nsh(
    flags: &str,
    shader_names: &str,
    nsh_shader_dump: &str,
    cemu_shader_dump: &str,
) {
    let mut flags_values = Vec::new();
    for line in std::fs::read_to_string(flags).unwrap().lines() {
        flags_values.push(u32::from_str_radix(line, 16).unwrap());
    }

    // Read nsh binaries only once.
    let mut sm4sh_shaders = Vec::new();
    for entry in std::fs::read_dir(nsh_shader_dump).unwrap() {
        let sm4sh_path = entry.unwrap().path();
        if sm4sh_path.extension().and_then(|e| e.to_str()) == Some("bin") {
            let sm4sh_bytes = std::fs::read(&sm4sh_path).unwrap();
            sm4sh_shaders.push((sm4sh_path, sm4sh_bytes));
        }
    }

    // Each flags like 92000161 has a pixel shader name like "shader_8e2dda0cc310098f_0000000000000079" from Cemu in RenderDoc.
    // This matches a binary like 8e2dda0cc310098f_0000000000000079_ps.bin in the Cemu shader dump.
    // This compiled WiiU shader binary can then be used to find the shader index in texas_cross.nsh.
    // In practice, flags in order starting from 92000161 have increasing indices.
    // The gap between indices varies, so this needs to be precomputed using shader dumps.
    for (name, flags) in std::fs::read_to_string(shader_names)
        .unwrap()
        .lines()
        .zip(flags_values)
    {
        let name = name.trim().strip_prefix("shader_").unwrap();
        let path = Path::new(cemu_shader_dump).join(format!("{name}_ps.bin"));
        if let Ok(cemu_bytes) = std::fs::read(path) {
            for (sm4sh_path, sm4sh_bytes) in &sm4sh_shaders {
                for i in 0..sm4sh_bytes.len() {
                    if let Some(b2) = sm4sh_bytes.get(i..i + cemu_bytes.len()) {
                        if b2 == cemu_bytes {
                            let sm4sh_name = sm4sh_path.file_stem().unwrap().to_string_lossy();
                            println!("{flags:X?}, {name}, {sm4sh_name}");
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn create_shader_database(flags_shaders: &str, nsh_shader_dump: &str, output: &str) {
    let mut programs = BTreeMap::new();
    for line in std::fs::read_to_string(flags_shaders).unwrap().lines() {
        let parts: Vec<_> = line.split(",").map(|s| s.trim()).collect();
        let flags = parts[0].to_string();
        let nsh_index: usize = parts[2]
            .strip_prefix("texas_cross.")
            .unwrap()
            .parse()
            .unwrap();

        let txt_path = Path::new(nsh_shader_dump).join(format!("texas_cross.{nsh_index}.txt"));
        let text = std::fs::read_to_string(txt_path).unwrap();
        let header = gfx2::pixel_shader_header(&text).unwrap().1;

        let samplers = header
            .sampler_vars
            .into_iter()
            .map(|s| (s.location, s.name))
            .collect();

        // NU_ parameters are in the MC block.
        let mut parameters = BTreeMap::new();
        if let Some(block_index) = header.uniform_blocks.iter().position(|b| b.name == "MC") {
            for var in header.uniform_vars.into_iter() {
                if var.block == block_index {
                    parameters.insert(var.offset, var.name);
                }
            }
        }

        programs.insert(
            flags,
            ShaderProgram {
                samplers,
                parameters,
            },
        );
    }

    let database = ShaderDatabase { programs };
    let json = serde_json::to_string_pretty(&database).unwrap();
    std::fs::write(output, &json).unwrap();
}
