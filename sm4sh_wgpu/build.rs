use wgsl_to_wgpu::{MatrixVectorTypes, WriteOptions, create_shader_modules};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    write_shader(
        include_str!("src/shader/blit.wgsl"),
        "src/shader/blit.wgsl",
        format!("{out_dir}/blit.rs"),
    );
    write_shader(
        include_str!("src/shader/bloom_add.wgsl"),
        "src/shader/bloom_add.wgsl",
        format!("{out_dir}/bloom_add.rs"),
    );
    write_shader(
        include_str!("src/shader/bloom_blur_combine.wgsl"),
        "src/shader/bloom_blur_combine.wgsl",
        format!("{out_dir}/bloom_blur_combine.rs"),
    );
    write_shader(
        include_str!("src/shader/bloom_blur.wgsl"),
        "src/shader/bloom_blur.wgsl",
        format!("{out_dir}/bloom_blur.rs"),
    );
    write_shader(
        include_str!("src/shader/bloom_bright.wgsl"),
        "src/shader/bloom_bright.wgsl",
        format!("{out_dir}/bloom_bright.rs"),
    );
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
