use std::{io::Cursor, path::Path};

use binrw::{BinRead, BinWrite};
use clap::Parser;
use rayon::prelude::*;
use sm4sh_lib::{nud::Nud, nut::Nut, vbn::Vbn};
use sm4sh_model::nud::NudModel;

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

    #[arg(long)]
    nud_model: bool,

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

    if cli.nud_model || cli.all {
        println!("Checking Nud models...");
        check_all(root, &["*.nud"], check_nud_model);
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
}

fn check_nud_model(nud: Nud, path: &Path, original_bytes: &[u8]) {
    let nut_path = path.with_file_name("model.nut");
    match Nut::from_file(&nut_path) {
        Ok(nut) => {
            let model = NudModel::from_nud(&nud, &nut).unwrap();

            // Check nud model conversions.
            let new_nud = model.to_nud().unwrap();

            if new_nud.vertex_buffer0 != nud.vertex_buffer0 {
                println!("Vertex buffer0 read/write not 1:1 for {path:?}");
            }
            if new_nud.vertex_buffer1 != nud.vertex_buffer1 {
                println!("Vertex buffer1 read/write not 1:1 for {path:?}");
            }
            if new_nud.index_buffer != nud.index_buffer {
                println!("Vertex indices read/write not 1:1 for {path:?}");
            }
        }
        Err(e) => println!("Error reading Nut from {nut_path:?}: {e}"),
    }
}

fn check_nut(nut: Nut, path: &Path, original_bytes: &[u8]) {
    let mut writer = Cursor::new(Vec::new());
    nut.write(&mut writer).unwrap();
    if writer.into_inner() != original_bytes {
        println!("Nut read/write not 1:1 for {path:?}");
    }
}

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
