extern crate glsl_to_spirv;

use glsl_to_spirv::ShaderType;
// use std::env;
use std::error::Error;
use std::path::Path;
use std::{fs, io};

fn main() -> Result<(), Box<dyn Error>> {
    // Check debug or release build
    // if Ok("release".to_owned()) == env::var("PROFILE") {
    //     copy_dir_all("res/", "build/release/")?;
    // } else {
    //     copy_dir_all("res/", "build/debug/")?;
    // }

    // Change detection of source shaders
    println!("cargo:rerun-if-changed=res/shaders/glsl");

    // Compile each shader at source
    for entry in std::fs::read_dir("res/shaders/glsl")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let in_path = entry.path();

            // Match shader file extension
            if let Some(shader_type) =
                in_path
                    .extension()
                    .and_then(|ext| match ext.to_string_lossy().as_ref() {
                        "vert" => Some(ShaderType::Vertex),
                        "frag" => Some(ShaderType::Fragment),
                        _ => None, // TODO! Other Shadertypes!
                    })
            {
                use std::io::Read;

                let source = std::fs::read_to_string(&in_path)?;

                let mut compiled_file = glsl_to_spirv::compile(&source, shader_type)?;

                let mut compiled_bytes = Vec::new();
                compiled_file.read_to_end(&mut compiled_bytes)?;

                let out_path = format!(
                    "res/shaders/spirv/{}.spv",
                    in_path.file_name().unwrap().to_string_lossy()
                );

                std::fs::write(&out_path, &compiled_bytes)?;
            }
        }
    }

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
