use std::{env, fs::OpenOptions, io::Write};

fn main() {
    if env::var("CARGO_FEATURE_LOGIND").ok().is_none() && env::var("CARGO_FEATURE_LIBSEAT").ok().is_none() {
        println!("cargo:warning=You are compiling without logind/libseat support.");
        println!(
            "cargo:warning=This means that you'll likely need to run it as root if you want to launch it from a tty."
        );
        println!("cargo:warning=To enable logind support add `--feature logind` to your cargo invocation.");
        println!("cargo:warning=$ cargo run --feature logind");
        println!("cargo:warning=To enable libseat support add `--feature libseat` to your cargo invocation.");
        println!("cargo:warning=$ cargo run --feature libseat");
    }

    // Shader compilation
    {
        let shader_path = env::current_dir()
            .unwrap()
            .join("src")
            .join("vulkan")
            .join("renderer")
            .join("shader");

        let mut compiler = shaderc::Compiler::new().unwrap();

        // Vertex shader
        let vertex_shader = include_str!("src/vulkan/renderer/shader/vert.glsl");
        let compiled_vertex = compiler
            .compile_into_spirv(&vertex_shader, shaderc::ShaderKind::Vertex, "vert.glsl", "main", None)
            .unwrap();
        {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(false)
                .open(shader_path.join("vert.spv"))
                .unwrap();
            file.write_all(compiled_vertex.as_binary_u8()).unwrap();
        }
    }
}
