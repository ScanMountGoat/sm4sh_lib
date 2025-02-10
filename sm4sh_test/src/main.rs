use std::{
    io::{Cursor, Seek},
    path::Path,
};

use binrw::{BinRead, BinWrite};
use clap::Parser;
use rayon::prelude::*;
use sm4sh_lib::{nud::Nud, nut::Nut, vbn::Vbn};
use sm4sh_model::nud::vertex::{read_vertices, write_vertices};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The root folder to test.
    root_folder: String,

    #[arg(long)]
    nud: bool,

    #[arg(long)]
    nut: bool,

    #[arg(long)]
    vbn: bool,

    /// Process all file types.
    #[arg(long)]
    all: bool,
}

fn main() {
    let cli = Cli::parse();
    let root = Path::new(&cli.root_folder);

    let start = std::time::Instant::now();

    if cli.nud || cli.all {
        println!("Checking Nud files...");
        check_all(root, &["*.nud"], check_nud);
    }

    if cli.nut || cli.all {
        println!("Checking Nut files...");
        check_all(root, &["*.nut"], check_nut);
    }

    if cli.vbn || cli.all {
        println!("Checking Vbn files...");
        check_all(root, &["*.vbn"], check_vbn);
    }

    println!("Finished in {:?}", start.elapsed());
}

fn check_all<T, F>(root: &Path, patterns: &[&str], check_file: F)
where
    for<'a> T: BinRead<Args<'a> = ()>,
    F: Fn(T, &Path, &[u8]) + Sync,
{
    globwalk::GlobWalkerBuilder::from_patterns(root, patterns)
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let original_bytes = std::fs::read(path).unwrap();
            let mut reader = Cursor::new(&original_bytes);
            match T::read_be(&mut reader) {
                Ok(file) => {
                    check_file(file, path, &original_bytes);
                }
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

// TODO: Make this a trait?
fn check_nud(nud: Nud, path: &Path, original_bytes: &[u8]) {
    let mut writer = Cursor::new(Vec::new());
    nud.write(&mut writer).unwrap();
    if writer.into_inner() != original_bytes {
        println!("Nud read/write not 1:1 for {path:?}");
    }

    // TODO: Check nud model conversions.

    let mut new_buffer0 = Cursor::new(Vec::new());
    let mut new_buffer1 = Cursor::new(Vec::new());

    let mut index = 0;
    for group in &nud.mesh_groups {
        for i in 0..group.mesh_count {
            let mesh = &nud.meshes[index];

            // Test read/write for vertex buffers.
            // TODO: Avoid indexing panics.
            let buffer0 = &nud.vertex_buffer0[mesh.vertex_buffer0_offset as usize..];
            let buffer1 = &nud.vertex_buffer1[mesh.vertex_buffer1_offset as usize..];

            match read_vertices(
                buffer0,
                buffer1,
                mesh.vertex_flags,
                mesh.uv_color_flags,
                mesh.vertex_count,
            ) {
                Ok(vertices) => {
                    let (vertex_flags, uv_color_flags) =
                        write_vertices(&vertices, &mut new_buffer0, &mut new_buffer1).unwrap();
                    if vertex_flags != mesh.vertex_flags || uv_color_flags != mesh.uv_color_flags {
                        println!("Flags not 1:1 for mesh {}[{i}], {path:?}", &group.name);
                    }
                }
                Err(e) => println!(
                    "Error reading vertices for mesh {}[{i}], {path:?}: {e}",
                    &group.name
                ),
            }

            index += 1;
        }
    }

    // TODO: Move this to sm4sh_model.
    let size = new_buffer0.stream_position().unwrap();
    align(&mut new_buffer0, size, 16, 0u8).unwrap();

    let size = new_buffer1.stream_position().unwrap();
    align(&mut new_buffer1, size, 16, 0u8).unwrap();

    if &new_buffer0.into_inner() != &nud.vertex_buffer0 {
        println!("Vertex buffer0 read/write not 1:1 for {path:?}");
    }
    if &new_buffer1.into_inner() != &nud.vertex_buffer1 {
        // TODO: This still isn't correct for some models?
        println!("Vertex buffer1 read/write not 1:1 for {path:?}");
    }
}

fn align<W: std::io::Write>(
    writer: &mut W,
    size: u64,
    align: u64,
    pad: u8,
) -> Result<(), std::io::Error> {
    let aligned_size = size.next_multiple_of(align);
    let padding = aligned_size - size;
    writer.write_all(&vec![pad; padding as usize])?;
    Ok(())
}

fn check_nut(nut: Nut, path: &Path, original_bytes: &[u8]) {}

fn check_vbn(vbn: Vbn, path: &Path, original_bytes: &[u8]) {
    if !write_le_bytes_equals(&vbn, original_bytes) {
        println!("Vbn read/write not 1:1 for {path:?}");
    }
}

fn write_le_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    for<'a> T: BinWrite<Args<'a> = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    value.write_le(&mut writer).unwrap();
    writer.into_inner() == original_bytes
}
