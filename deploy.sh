#!/bin/bash

# First on raspberry pi open terminal and run: screen -S mysession

cargo build --release --target aarch64-unknown-linux-gnu

echo -e "Copying shaders to Raspberry Pi..."
# scp ./res/shaders/master.frag \
#     ./res/shaders/master.vert \
#     mattszymonski@192.168.33.17:/home/mattszymonski/programming/shader-editor-rs/res/shaders/


# scp ./res/shaders/compiled/master.frag.spv \
#     ./res/shaders/compiled/master.vert.spv \
#     mattszymonski@192.168.33.17:/home/mattszymonski/programming/shader-editor-rs/res/shaders/compiled/



echo -e "Copying binary to Raspberry Pi..."
sshpass -p "ras" scp ./target/aarch64-unknown-linux-gnu/release/little-shader-display \
    mattszymonski@192.168.33.17:/home/mattszymonski/programming/little-shader-display/

echo -e "Running little-shader-display..."
sshpass -p 'ras' ssh mattszymonski@192.168.33.17 "/usr/bin/screen -S mysession -X stuff '/home/mattszymonski/programming/little-shader-display/little-shader-display -- --window\n'"





#sshpass -p 'ras' ssh mattszymonski@192.168.33.17 "/usr/bin/screen -S mysession -X stuff 'glxinfo | grep -iE 'OpenGL|renderer|version'\n'"

# sshpass -p 'ras' ssh mattszymonski@192.168.33.17 "/usr/bin/screen -S mysession -X stuff 'cmake -DILI9341=OFF \
#       -DST7789=ON \
#       -DSPI_BUS_CLOCK_DIVISOR=6 \
#       -DGPIO_TFT_DATA_CONTROL=25 \
#       -DGPIO_TFT_RESET=27 \
#       -DGPIO_TFT_BACKLIGHT=18 \
#       -DARMV6Z=OFF \
#       ..\n'"


# sshpass -p 'ras' ssh mattszymonski@192.168.33.17 "/usr/bin/screen -S mysession -X stuff 'sudo apt install -y \
#     libegl1-mesa-dev \
#     libgles2-mesa-dev \
#     libgl1-mesa-dev \
#     libglvnd-dev \
#     libdrm-dev \
#     libgbm-dev \
#     mesa-utils \
#     libwayland-dev \
#     libx11-dev
# \n'"




# scp .\glslc mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display
# scp .\little-shader-display mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display
# scp .\master.vert mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display/res/shaders
# scp .\master.frag mattszymonski@192.168.33.14:/home/mattszymonski/programming/little-shader-display/res/shaders