use wgsl_to_wgpu::{create_shader_module_embedded, MatrixVectorTypes, WriteOptions};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    write_shader(
        include_str!("src/shader/model.wgsl"),
        "src/shader/model.wgsl",
        format!("{out_dir}/model.rs"),
    );
}

fn write_shader(wgsl_source: &str, wgsl_path: &str, output_path: String) {
    println!("cargo:rerun-if-changed={wgsl_path}");

    // Generate the Rust bindings and write to a file.
    let text = create_shader_module_embedded(
        wgsl_source,
        WriteOptions {
            derive_bytemuck_vertex: true,
            derive_encase_host_shareable: true,
            matrix_vector_types: MatrixVectorTypes::Glam,
            ..Default::default()
        },
    )
    .unwrap();

    std::fs::write(output_path, text.as_bytes()).unwrap();
}
