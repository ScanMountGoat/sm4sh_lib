use std::{io::Cursor, path::Path};

use binrw::{BinRead, BinWrite};
use clap::Parser;
use rayon::prelude::*;
use sm4sh_lib::{
    jtb::Jtb, mta::Mta, nhb::Nhb, nud::Nud, nut::Nut, omo::Omo, pack::Pack, sb::Sb, vbn::Vbn,
};
use sm4sh_model::{animation::Animation, NudModel};

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
    pack: bool,

    #[arg(long)]
    mta: bool,

    #[arg(long)]
    nhb: bool,

    #[arg(long)]
    jtb: bool,

    #[arg(long)]
    sb: bool,

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

    if cli.pack || cli.all {
        println!("Checking Pack files...");
        check_all(root, &["*.pac"], check_pack);
    }

    if cli.mta || cli.all {
        println!("Checking Mta files...");
        check_all(root, &["*.mta"], check_mta);
    }

    if cli.nhb || cli.all {
        println!("Checking Nhb files...");
        check_all(root, &["*.nhb"], check_nhb);
    }

    if cli.jtb || cli.all {
        println!("Checking Jtb files...");
        check_all(root, &["*.jtb"], check_jtb);
    }

    if cli.sb || cli.all {
        println!("Checking Sb files...");
        check_all(root, &["*.sb"], check_sb);
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
            // TODO: Why are some mta files empty?
            if !original_bytes.is_empty() {
                let mut reader = Cursor::new(&original_bytes);
                match T::read_be(&mut reader) {
                    Ok(file) => {
                        check_file(file, path, &original_bytes);
                    }
                    Err(e) => println!("Error reading {path:?}: {e}"),
                }
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

fn check_nud_model(nud: Nud, path: &Path, _original_bytes: &[u8]) {
    let nut = Nut::from_file(path.with_file_name("model.nut")).ok();

    let vbn = Vbn::from_file(path.with_file_name("model.vbn")).ok();
    match NudModel::from_nud(&nud, nut.as_ref(), vbn.as_ref()) {
        Ok(model) => {
            // Check nud model conversions.
            let new_nud = model.to_nud().unwrap();

            if new_nud.bone_start_index != nud.bone_start_index
                || new_nud.bone_end_index != nud.bone_end_index
            {
                println!("Bone index range not 1:1 for {path:?}",);
            }

            for (group_old, group_new) in nud.mesh_groups.iter().zip(&new_nud.mesh_groups) {
                if group_old.bone_flags != group_new.bone_flags {
                    println!("Group bone flags not 1:1 for {path:?}",);
                }
            }

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
        Err(e) => println!("Error converting {path:?}: {e}"),
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

fn check_pack(pack: Pack, path: &Path, original_bytes: &[u8]) {
    if !write_be_bytes_equals(&pack, original_bytes) {
        println!("Pack read/write not 1:1 for {path:?}");
    }

    for item in pack.items {
        if !item.data.is_empty() {
            if item.name.ends_with(".omo") {
                match Omo::from_bytes(&item.data) {
                    Ok(omo) => check_omo(omo, path, &item.data),
                    Err(e) => println!("Error reading {} for {path:?}: {e}", item.name),
                }
            } else if item.name.ends_with("mta") {
                match Mta::from_bytes(&item.data) {
                    Ok(mta) => check_mta(mta, path, &item.data),
                    Err(e) => println!("Error reading {} for {path:?}: {e}", item.name),
                }
            }
        }
    }
}

fn check_omo(omo: Omo, path: &Path, original_bytes: &[u8]) {
    let mut writer = Cursor::new(Vec::new());
    omo.write(&mut writer).unwrap();
    if writer.into_inner() != original_bytes {
        println!("Omo read/write not 1:1 for {path:?}");
    }

    if let Err(e) = Animation::from_omo(&omo) {
        println!("Error loading animation for {path:?}: {e}")
    }
}

fn check_mta(mta: Mta, path: &Path, original_bytes: &[u8]) {
    let mut writer = Cursor::new(Vec::new());
    mta.write(&mut writer).unwrap();
    if writer.into_inner() != original_bytes {
        println!("Mta read/write not 1:1 for {path:?}");
    }
}

fn check_nhb(nhb: Nhb, path: &Path, original_bytes: &[u8]) {
    let mut writer = Cursor::new(Vec::new());
    nhb.write(&mut writer).unwrap();
    if writer.into_inner() != original_bytes {
        println!("Nhb read/write not 1:1 for {path:?}");
    }
}

fn check_jtb(jtb: Jtb, path: &Path, original_bytes: &[u8]) {
    if !write_be_bytes_equals(&jtb, original_bytes) {
        println!("Jtb read/write not 1:1 for {path:?}");
    }
}

fn check_sb(sb: Sb, path: &Path, original_bytes: &[u8]) {
    if !write_le_bytes_equals(&sb, original_bytes) {
        println!("Sb read/write not 1:1 for {path:?}");
    }
}

fn write_be_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    for<'a> T: BinWrite<Args<'a> = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    value.write_be(&mut writer).unwrap();
    writer.into_inner() == original_bytes
}

fn write_le_bytes_equals<T>(value: &T, original_bytes: &[u8]) -> bool
where
    for<'a> T: BinWrite<Args<'a> = ()>,
{
    let mut writer = Cursor::new(Vec::new());
    value.write_le(&mut writer).unwrap();
    writer.into_inner() == original_bytes
}
