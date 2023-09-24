extern crate glsl_to_spirv;

use glsl_to_spirv::ShaderType;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
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
