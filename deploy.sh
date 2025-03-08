#!/bin/bash

cargo build --release --target aarch64-unknown-linux-gnu

scp ./res/shaders/master.frag \
    ./res/shaders/master.vert \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/res/shaders/

scp ./res/shaders/compiled/master.frag.spv \
    ./res/shaders/compiled/master.vert.spv \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/res/shaders/compiled/

scp ./target/aarch64-unknown-linux-gnu/release/shader-editor-rs \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/

# scp .\glslc mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display
# scp .\little-shader-display mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display
# scp .\master.vert mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display/res/shaders
# scp .\master.frag mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display/res/shaders