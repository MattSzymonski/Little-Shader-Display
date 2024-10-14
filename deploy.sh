#!/bin/bash

scp ./res/shaders/master.frag \
    ./res/shaders/master.vert \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/res/shaders/

scp ./res/shaders/compiled/master.frag.spv \
    ./res/shaders/compiled/master.vert.spv \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/res/shaders/compiled/

scp ./target/aarch64-unknown-linux-gnu/release/shader-editor-rs \
    mattszymonski@192.168.0.130:/home/mattszymonski/programming/shader-editor-rs/