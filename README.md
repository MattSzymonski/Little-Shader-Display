# Little Shader Display
Shader display written in Rust
- Meant to run on Raspberry Pi Zero W 2
- Image is outputted to both window and ST7789 display connected via GPIO and SPI
- Shader hot-reloading is implemented

## Installation
Cross-compilation is required since Raspberry Pi Zero W 2 is too weak to compile the program on its own.

Setup cross-compilation toolchain as follows:
```
sudo dnf install dbus-devel pkgconf-pkg-config
sudo apt install libdbus-1-dev pkg-config
sudo apt install libdbus-1-dev:arm64
dpkg --print-foreign-architectures
export PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu
export PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
export PKG_CONFIG_ALLOW_CROSS=1
```

1. Building on build machine (once): 
    1. git clone https://github.com/MattSzymonski/Little-Shader-Display
    2. Install glslc shader compiler using `sudo apt-get install glslc` or download it [here](https://storage.googleapis.com/shaderc/badges/build_link_linux_gcc_release.html)
    3. Setup cross-compilation toolchain as described above
    4. Build using `cargo build --release --target aarch64-unknown-linux-gnu`
    5. Copy shaders and output program to Raspberry Pi board using `./deploy.sh` (adjust paths, user and ip address before)
2. Running on Raspberry Pi board:
    1. Connect ST7789 screen to the pins as presented [here](https://www.waveshare.com/wiki/1.69inch_LCD_Module)
    1. Install glslc shader compiler using `sudo apt-get install glslc` or download it [here](https://storage.googleapis.com/shaderc/badges/build_link_linux_gcc_release.html)
    2. Run the program using `./little-shader-display -- --window --st7789` (use `window` and `st7789` flags to choose the display) 
    3. Modify the shaders and have fun