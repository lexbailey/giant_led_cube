// Copied from: https://michael-f-bryan.github.io/rust-ffi-guide/cbindgen.html

extern crate cbindgen;

use std::{env, process::Command};
use std::path::PathBuf;
use cbindgen::{Config, Language};
use std::io::Write;

// TODO this is generated on the host, so it picks the sizes for the host machine
// if this is compiled on a 64 bit host, then it will probably be an over-estimate, which is fine
// regardless, this number could be wrong. Given it's probably an over-estimate it's fine for now
// need to get the target compiler to generate the sizes somehow, instead of the host compiler.
fn size_header_text() -> String {
    let mut result = String::new();
    for s in [
        format!("#define COLORS_ENUM_BYTES ({})\n", std::mem::size_of::<cube_model::Colors>())
        ,format!("#define SUBFACE_STRUCT_BYTES ({})\n", std::mem::size_of::<cube_model::SubFace>())
        ,format!("#define FACE_STRUCT_BYTES ({})\n", std::mem::size_of::<cube_model::Face>())
        ,format!("#define TWIST_STRUCT_BYTES ({})\n", std::mem::size_of::<cube_model::Twist>())
        ,format!("#define CUBE_STRUCT_BYTES ({})\n", std::mem::size_of::<cube_model::Cube>())
        ,format!("#define SWITCH_ARRAY_BYTES ({})\n", std::mem::size_of::<cube_model::SwitchMap5Faces>())
        ,format!("#define OUTPUT_ARRAY_BYTES ({})\n", std::mem::size_of::<cube_model::OutputMap5Faces>())
    ].iter() {
        result.push_str(s);
    }
    result
}

fn main() {
    // Run cbindgen
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let output_file = target_dir()
        .join("include")
        .join("cube_data.h")
        .display()
        .to_string();

    let config = Config {
        namespace: Some(String::from("ffi")),
        language: Language::C,
        include_guard: Some("CUBE_DATA_INCLUDE".into()),
        ..Default::default()
    };

    cbindgen::generate_with_config(&crate_dir, config)
      .unwrap()
      .write_to_file(&output_file);

    // Set GIT_VERSION environment variable
    let git_hash_output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(git_hash_output.stdout).unwrap();
    let git_modified = Command::new("git")
        .args(&["diff-index", "--quiet", "HEAD"])
        .status()
        .unwrap()
        .code().unwrap() != 0;

    let git_version = format!("{}{}", git_hash.trim(), if git_modified { "-modified" } else { "" });

    println!("cargo:rustc-env=GIT_VERSION={}", git_version);

    let sizes_output_file = target_dir()
        .join("include")
        .join("model_size_info.h")
        .display()
        .to_string();

    let mut f = std::fs::File::create(sizes_output_file).expect("failed to create size info file");
    let output = size_header_text();
    f.write(output.as_bytes());

}

/// Find the location of the `target/` directory. Note that this may be 
/// overridden by `cmake`, so we also need to check the `CARGO_TARGET_DIR` 
/// variable.
fn target_dir() -> PathBuf {
    if let Ok(target) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(target)
    } else {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target")
    }
}

