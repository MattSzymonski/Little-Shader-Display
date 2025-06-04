#!/bin/bash

# First on raspberry pi open terminal and run: screen -S mysession

cargo build --release --target aarch64-unknown-linux-gnu

echo -e "Copying shaders to Raspberry Pi..."
sshpass -p "ras" scp -rp ./res mattszymonski@192.168.33.17:/home/mattszymonski/programming/little-shader-display/

echo -e "Copying binary to Raspberry Pi..."
sshpass -p "ras" scp ./target/aarch64-unknown-linux-gnu/release/little-shader-display mattszymonski@192.168.33.17:/home/mattszymonski/programming/little-shader-display/

echo -e "Running little-shader-display..."
sshpass -p 'ras' ssh mattszymonski@192.168.33.17 "/usr/bin/screen -S mysession -X stuff '/home/mattszymonski/programming/little-shader-display/little-shader-display -- --st7789 --bluetooth\n'"
