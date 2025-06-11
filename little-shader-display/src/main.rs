// --- Module declarations and conditional compilation for platform-specific drivers ---
mod file_watcher;
mod bluetooth_server;
mod renderer;

#[cfg(target_os = "linux")]
mod st7789_driver;

// --- Standard and external library imports ---
use std::{
    env, 
    path::{PathBuf},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use renderer::Renderer;
use file_watcher::FileWatcher;
use tokio::sync::Mutex;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};
use std::io::Read;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
use bluetooth_server::BluetoothServer;

static SHADER_NAMES: [&str; 5] = ["waves.frag", "mutation.frag", "fractal.frag", "grid.frag", "rings.frag"];
static ST7789_OUTPUT_SIZE: u32 = 256;

static SHADERS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::current_exe().unwrap().parent().unwrap().join("res").join("shaders")
});

static COMPILED_VERTEX_SHADER_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    SHADERS_PATH.join("compiled").join("master.vert.spv")
});

static COMPILED_FRAGMENT_SHADER_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    SHADERS_PATH.join("compiled").join("master.frag.spv")
});


#[tokio::main]
async fn main() {
    let mut use_window = false;
    let mut use_st7789 = false;
    let mut use_bluetooth = false;

    // --- Parse command-line arguments ---

    let args: Vec<String> = env::args().collect();
    for arg in &args {
        match arg.as_str() {
            "--window" => use_window = true,
            "--st7789" => use_st7789 = true,
            "--bluetooth" => use_bluetooth = true,
            _ => {}
        }
    }

    println!("Using window display: {}", use_window);
    println!("Using st7789 display: {}", use_st7789);
    println!("Using bluetooth: {}", use_bluetooth);

    if use_st7789 && cfg!(target_os = "windows") {
        panic!("st7789 display is not supported on Windows");
    }

    if !use_window && cfg!(target_os = "windows") {
        panic!("No display chosen for Windows");
    }

    if !use_window && !use_st7789 && cfg!(target_os = "linux") {
        panic!("No display chosen for Linux");
    }

    // --- Create st7789 driver, window, renderer, file watcher, and bluetooth server ---

    // Create and initialize st7789 driver if requested and on Linux 
    #[cfg(target_os = "linux")]
    let st7789_driver: Option<st7789_driver::RaspberryST7789Driver> = if use_st7789 {
        let mut driver = st7789_driver::RaspberryST7789Driver::new().unwrap();
        driver.initialize().unwrap();
        Some(driver)
    } else {
        None
    };

    // Create window if requested
    let mut event_loop = EventLoop::new(); 
    let window: Option<Window> = if use_window {
        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(500, 500))
            .with_title("Little Shader Display")
            .with_visible(true) // Make visible directly
            .build(&event_loop)
            .expect("failed to create a window");
        Some(window)
    } else {
        None
    };

    // Create a file watcher to monitor shader files for changes
    let mut file_watcher = FileWatcher::new(std::env::current_exe().unwrap().parent().unwrap().join(SHADERS_PATH.clone().join("uncompiled")));
   
    // Only on Linux: include all arguments
    #[cfg(target_os = "linux")]
    let mut renderer = Renderer::new(use_window, window.as_ref(), use_st7789, st7789_driver);

    // On other platforms
    #[cfg(not(target_os = "linux"))]
    let mut renderer = Renderer::new(use_window, window.as_ref());

    let bluetooth_server: Option<Arc<Mutex<Option<String>>>> = if use_bluetooth {
        let server = BluetoothServer::new().await.unwrap();
        let received_text = server.received_text.clone();
    
        tokio::spawn(async move {
            server.run().await.unwrap();
        });
    
        Some(received_text)
    } else {
        None
    };

    // --- Define main loop variables ---

    let mut current_shader_index = 0;
    let start_time = Instant::now();
    let mut running = true;
    let mut frame = 0;

    
    let mut last_fps_update = Instant::now();
    
    // Setup non-blocking stdin reading to detect user input 
    let stdin = File::open("/dev/stdin").unwrap();
    let fd: i32 = stdin.as_raw_fd();
    let flags = unsafe { fcntl(fd, F_GETFL) };
    unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) };    

    let mut bluetooth_data = String::new();

    // --- Main loop ---

    println!("Initialization complete. Starting main loop...");

    while running {
        frame += 1;

        // 1. Check for data received by bluetooth server
        if use_bluetooth {
            // Check if the Bluetooth server is running and print the latest received message
            if let Some(received_text) = &bluetooth_server {
                if let Ok(message) = received_text.try_lock() {
                    if let Some(ref string) = *message {
                        bluetooth_data = string.clone();
                    }
                } 
            }
        }

        // 2. Handle window events
        if use_window {
            running = handle_window_event(&mut event_loop, &mut renderer);
        }

        // 3. Handle user input to switch shaders
        let mut buffer = [0u8; 1];
        if stdin.try_clone().unwrap().read(&mut buffer).is_ok() {
            if buffer[0] == b' ' {
                current_shader_index = (current_shader_index + 1) % SHADER_NAMES.len();
                println!("Switched to shader index: {}", current_shader_index);
                renderer.recompile_shaders(current_shader_index, true, true);
            }
        }

        // 4. Calculate elapsed time
        let elapsed_time = start_time.elapsed().as_secs_f32();
        
        // 5. Update uniform buffer with the new values
        renderer.update_uniforms(elapsed_time, bluetooth_data.clone());

        // 6. FPS Calculation: Print FPS every second
        if last_fps_update.elapsed() >= Duration::from_secs(1) {
            println!("FPS: {}", frame);
            frame = 0; // Reset counter
            last_fps_update = Instant::now(); // Reset timer
        }

        // 7. Check for shader file changes, recompile them and recreate pipeline if necessary
        if let Some(paths) = file_watcher.get_changes() {
            for path in paths {
                let file_name = path.file_name().unwrap();
                println!("Shader file change detected: {:?}. Name: {:?}", path, file_name);
    
                // Check if the changed file is a vertex
                if file_name.to_str().unwrap().ends_with(".vert") {
                    renderer.recompile_shaders(current_shader_index, true, false);
                }
    
                // Check if the changed file is a fragment shader
                if file_name.to_str().unwrap().ends_with(".frag") {
                    renderer.recompile_shaders(current_shader_index, false, true);
                }
            }
        }

        // 8. Render
        renderer.render();
    }
}

fn handle_window_event(
    event_loop: &mut EventLoop<()>,
    renderer: &mut Renderer,
) -> bool {
    let mut running: bool = true;

    event_loop.run_return(|event, _, control_flow| {
        control_flow.set_wait();

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Resized(size) => {
                    renderer.resize(size.width, size.height);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(new_inner_size.width, new_inner_size.height);
                }
                _ => (),
            },
            Event::MainEventsCleared => control_flow.set_exit(),
            _ => (),
        }
    });

    return running;
}


