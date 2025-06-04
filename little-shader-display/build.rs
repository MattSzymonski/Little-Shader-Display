// This script is run by cargo on build
use std::process::Command;
use std::env;

fn main()  {

    // This tells cargo to rerun this script if something in /res/shaders changes.
    println!("cargo:rerun-if-changed=res/shaders/*");

    // Compile GLSL shaders into SPIR-V
    let shader_compiler_path = if cfg!(target_os = "windows") {
        "./glslc.exe" // Windows executable
    } else {
        "./glslc" // Linux executable
    };
    let shader_directory_path = "res/shaders";
    let built_shader_directory_path = "res/shaders/compiled";

    let shaders_to_compile = [
        "master.vert", 
        "master.frag",
    ];

    for shader_to_compile in shaders_to_compile.iter() {
        let compiled_shader_file_name = format!("{}{}", shader_to_compile, ".spv");
        let command = format!("{} {}/{} -o {}/{}", shader_compiler_path, shader_directory_path, shader_to_compile, built_shader_directory_path, compiled_shader_file_name);

        if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command.as_ref()])
            .output()
            .expect(&format!("Failed to compile shader: {}", shader_to_compile));
        } else {
            Command::new("sh")
            .arg("-c") // Use -c to execute the command string
            .arg(command) // Pass the command as a string
            .output()
            .expect(&format!("Failed to compile shader: {}", shader_to_compile));
        }
    }

}