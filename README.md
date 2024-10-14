# Shader Editor RS
Shader editor written in Rust
- Meant to run on Raspberry Pi Zero W 2
- Image is outputted to both window and ST7789 display connected via GPIO and SPI
- Shader hot-reloading is implemented

## Installation
Cross-compilation is required since Raspberry Pi Zero W 2 is too weak to compile the program.
1. Building on build machine (once): 
    1. git clone https://github.com/MattSzymonski/Shader-Editor-RS
    2. Install glslc shader compiler using `sudo apt-get install glslc` or download it [here](https://storage.googleapis.com/shaderc/badges/build_link_linux_gcc_release.html)
    3. Build using `cargo build --release --target aarch64-unknown-linux-gnu`
    4. Copy shaders and output to Raspberry Pi board using `./deploy.sh` (adjust paths, user and ip address before)
2. Running on Raspberry Pi board:
    1. Connect ST7789 screen to the pins as presented [here](https://www.waveshare.com/wiki/1.69inch_LCD_Module)
    1. Install glslc shader compiler using `sudo apt-get install glslc` or download it [here](https://storage.googleapis.com/shaderc/badges/build_link_linux_gcc_release.html)
    2. ./shader-editor-rs
    3. Modify the shaders