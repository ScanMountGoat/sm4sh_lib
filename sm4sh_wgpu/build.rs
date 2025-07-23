use wgsl_to_wgpu::{create_shader_modules, MatrixVectorTypes, WriteOptions};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    write_shader(
        include_str!("src/shader/bone.wgsl"),
        "src/shader/bone.wgsl",
        format!("{out_dir}/bone.rs"),
    );
    write_shader(
        include_str!("src/shader/model.wgsl"),
        "src/shader/model.wgsl",
        format!("{out_dir}/model.rs"),
    );
}

fn write_shader(wgsl_source: &str, wgsl_path: &str, output_path: String) {
    println!("cargo:rerun-if-changed={wgsl_path}");

    // Generate the Rust bindings and write to a file.
    let text = create_shader_modules(
        wgsl_source,
        WriteOptions {
            derive_bytemuck_vertex: true,
            derive_encase_host_shareable: true,
            matrix_vector_types: MatrixVectorTypes::Glam,
            ..Default::default()
        },
        wgsl_to_wgpu::demangle_identity,
    )
    .inspect_err(|error| error.emit_to_stderr(wgsl_source))
    .map_err(|_| "Failed to validate shader")
    .unwrap();

    std::fs::write(output_path, text.as_bytes()).unwrap();
}
