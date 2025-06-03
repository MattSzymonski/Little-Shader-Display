// --- Module declarations and conditional compilation for platform-specific drivers ---
mod file_watcher;

#[cfg(target_os = "linux")]
mod raspberry_st7789_driver;

// --- Standard and external library imports ---
use std::{
    env, 
    fs, 
    iter::{self, once}, 
    mem::{self, size_of}, 
    path::PathBuf, 
    process::Command, 
    sync::LazyLock, 
    time::{Duration}
};
use std::time::Instant;
use bytemuck::cast_slice;
use bytemuck_derive::{Pod, Zeroable};
use file_watcher::FileWatcher;
use futures::executor::block_on;
use image::{ImageBuffer, Rgba};
use wgpu::{
    util::DeviceExt, 
    BufferDescriptor, 
    BufferUsages, 
    Color, 
    ColorTargetState, 
    CommandEncoderDescriptor,
    Device, 
    DeviceDescriptor, 
    Features, 
    FragmentState, 
    LoadOp, 
    MultisampleState, 
    Operations,
    PipelineLayout, 
    PipelineLayoutDescriptor,
    PresentMode, 
    PrimitiveState,
    RenderPassColorAttachment, 
    RenderPassDescriptor, 
    RenderPipeline, 
    RenderPipelineDescriptor, 
    RequestAdapterOptions, 
    ShaderModule, 
    ShaderModuleDescriptor, 
    SurfaceConfiguration, 
    TextureFormat, 
    TextureUsages, 
    TextureViewDescriptor,
    VertexAttribute, 
    VertexBufferLayout,
    VertexFormat, 
    VertexState, 
    VertexStepMode
};
use std::path::Path;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState},
    event_loop::{EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder},
};


// --- Data Structures for Rendering ---

// Uniform buffer struct that holds the current time to pass to the shader.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    time: f32,
}

impl Uniforms {
    fn new() -> Self {
        Self { time: 0.0 }
    }
}

// Vertex struct representing a position and its corresponding texture coordinate.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    texture_coordinates: [f32; 2]
}

impl Vertex {
    // Creates a new vertex with given position and texture coordinates.
    fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self { position: [x, y], texture_coordinates: [u, v] }
    }

    // Returns the layout description for the vertex buffer.
    fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                // Position attribute: location 0
                VertexAttribute { 
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // Texture coordinate attribute: location 1
                VertexAttribute { // Vertex texture coordinates
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// Vertices of two screen filling triangles
static VERTICES: LazyLock<[Vertex; 6]> = LazyLock::new(|| [
    // First triangle (top-left to bottom-right)
    Vertex::new(-1.0, 1.0, 0.0, 1.0),    // Top-left
    Vertex::new(-1.0, -1.0, 0.0, 0.0),   // Bottom-left
    Vertex::new(1.0, 1.0, 1.0, 1.0),     // Top-right
    
    // Second triangle (bottom-left to bottom-right)
    Vertex::new(1.0, 1.0, 1.0, 1.0),     // Top-right (shared vertex)
    Vertex::new(-1.0, -1.0, 0.0, 0.0),   // Bottom-left (shared vertex)
    Vertex::new(1.0, -1.0, 1.0, 0.0),    // Bottom-right
]);

fn main() {
    let mut use_window = false;
    let mut use_st7789 = false;
    
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    for arg in &args {
        match arg.as_str() {
            "--window" => use_window = true,
            "--st7789" => use_st7789 = true,
            _ => {}
        }
    }

    // Print selected options
    println!("Using window display: {}", use_window);
    println!("Using st7789 display: {}", use_st7789);

    if use_st7789 && cfg!(target_os = "windows") {
        panic!("st7789 display is not supported on Windows");
    }

    if !use_window && cfg!(target_os = "windows") {
        panic!("No display chosen for Windows");
    }

    if !use_window && !use_st7789 && cfg!(target_os = "linux") {
        panic!("No display chosen for Linux");
    }
    
    let output_size: u32 = 256;
    let shaders_path = env::current_dir().unwrap().join("res").join("shaders");
    let vertex_shader_path = shaders_path.join("master.vert");
    let fragment_shader_path = shaders_path.join("master.frag");
    let compiled_vertex_shader_path = shaders_path.join("compiled").join("master.vert.spv");
    let compiled_fragment_shader_path = shaders_path.join("compiled").join("master.frag.spv");

    // Create and initialize st7789 driver if requested and on Linux 
    #[cfg(target_os = "linux")]
    let mut st7789 = if use_st7789 {
        let mut driver = raspberry_st7789_driver::RaspberryST7789Driver::new().unwrap();
        driver.initialize().unwrap();
        Some(driver)
    } else {
        None
    };

    // Create window if requested
    let mut event_loop = EventLoop::new(); 
    let window: Option<Window> = if use_window {
        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_title("Little Shader Display")
            .with_visible(true) // Make visible directly
            .build(&event_loop)
            .expect("failed to create a window");
        Some(window)
    } else {
        None
    };

    // Create a file watcher to monitor shader files for changes
    let mut file_watcher = FileWatcher::new(env::current_dir().unwrap().join(shaders_path));
    let start_time = Instant::now();

    // --- Create GPU resources for rendering ---

    // 1. Initialize wgpu  
    let (device, queue, surface, mut surface_config, output_format) = match &window {
        Some(window) => initialize_wgpu_with_window(window),
        None => initialize_wgpu_no_window(),
    };

    // 2. Create uniform buffer
    let mut uniforms = Uniforms::new();
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // 3. Create a bind group layout for uniforms
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT, // or VERTEX if it's in the vertex shader
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // 4. Create a bind group from the layout and uniform buffer
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("uniform_bind_group"),
    });

    // 5. Define pipeline layout with uniform bindings
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // 6. Compile and create shaders
    compile_shader(vertex_shader_path.clone(), compiled_vertex_shader_path.clone());
    let mut vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("master_vertex_shader"),
        source: wgpu::util::make_spirv(&std::fs::read(compiled_vertex_shader_path.clone()).expect("Failed to read shader file")),
    });

    compile_shader(fragment_shader_path.clone(), compiled_fragment_shader_path.clone());
    let mut fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("master_fragment_shader"),
        source: wgpu::util::make_spirv(&std::fs::read(compiled_fragment_shader_path.clone()).expect("Failed to read shader file")),
    });

    // 7. Create a render pipeline using the shaders
    let mut render_pipeline = create_render_pipeline(&device, &pipeline_layout, &output_format, &vertex_shader, &fragment_shader);

    // 8. Upload vertex buffer data
    let vbo = device.create_buffer(&BufferDescriptor {
        label: None,
        size: size_of::<Vertex>() as u64 * 6,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vbo, 0, cast_slice(&*VERTICES));

    // 9. Create offscreen texture for rendering (used by ST7789 to read pixels)
    #[cfg(target_os = "linux")]
    let (output_image_texture, buffer) = if use_st7789 {
        let output_image_size = wgpu::Extent3d {
            width: output_size,
            height: output_size,
            depth_or_array_layers: 1,
        };

        let output_image_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: output_image_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: output_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
    
        let data_size = (output_size * output_size * 4) as u64; // 4 bytes per pixel (RGBA)

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Read Buffer"),
            size: data_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
    
        (Some(output_image_texture), Some(buffer))
    } else {
        (None, None)
    };

    // ------- Main loop -------

    let mut running = true;
    let mut frame = 0;
    let mut last_fps_update = Instant::now();
    
    while running {
        frame += 1;

        // 1. Handle window events
        if use_window {
            running = handle_window_event(running, &mut event_loop, &device, &surface.as_ref().unwrap(), &mut surface_config.as_mut().unwrap());
        }

        // 2. Calculate elapsed time
        let elapsed_time = start_time.elapsed().as_secs_f32();
        uniforms.time = elapsed_time;
        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // FPS Calculation: Print FPS every second
        if last_fps_update.elapsed() >= Duration::from_secs(1) {
            println!("FPS: {}", frame);
            frame = 0; // Reset counter
            last_fps_update = Instant::now(); // Reset timer
        }

        // 3. Check for shader file changes, recompile them and recreate pipeline if necessary
        check_and_reload_shaders(
            &mut file_watcher,
            &device,
            &pipeline_layout,
            &output_format,
            &vertex_shader_path,
            &compiled_vertex_shader_path,
            &fragment_shader_path,
            &compiled_fragment_shader_path,
            &mut vertex_shader,
            &mut fragment_shader,
            &mut render_pipeline,
        );

        // 4. Option: Render to the window surface
        if use_window {
            render_to_window(
                &device,
                &queue,
                surface.as_ref().unwrap(),
                surface_config.as_ref().unwrap(),
                &render_pipeline,
                &vbo,
                &bind_group,
            );
        }

        // 5. Option: Render to the ST7789 display
        #[cfg(target_os = "linux")]
        if use_st7789 {
            render_to_st7789(
                &device,
                &queue,
                &render_pipeline,
                &vbo,
                &bind_group,
                output_image_texture.as_ref().unwrap(),
                buffer.as_ref().unwrap(),
                st7789.as_mut().unwrap(),
            );
        }
    }
}

fn initialize_wgpu_no_window() -> (wgpu::Device, wgpu::Queue, Option<wgpu::Surface>, Option<wgpu::SurfaceConfiguration>, wgpu::TextureFormat) {
    
    // Create a wgpu instance without a window
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    // Create addapter without a surface
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None, 
        force_fallback_adapter: false,
    }))
    .expect("Failed to find a suitable adapter");
    println!("Using backend: {:?}, device: {}", adapter.get_info().backend, adapter.get_info().name);

    // Create device and queue
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: adapter.limits(),
        },
        None,
    ))
    .expect("Failed to create device");

    (device, queue, None, None, TextureFormat::Rgba8Unorm)
}

fn initialize_wgpu_with_window(window: &winit::window::Window) -> (wgpu::Device, wgpu::Queue, Option<wgpu::Surface>, Option<wgpu::SurfaceConfiguration>, wgpu::TextureFormat) {
    // Get the physical size of the window
    let physical_size = window.inner_size();

    // Create a wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    // Create a surface for the window
    let surface = unsafe { instance.create_surface(&window) }.expect("failed to create surface");

    // Create addapter with the surface
    let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("failed to find a suitable adapter");
    println!("Using backend: {:?}, device: {}", adapter.get_info().backend, adapter.get_info().name);

    // Create device and queue
    let (device, queue) = block_on(adapter.request_device(
        &DeviceDescriptor {
            label: None,
            features: Features::empty(),
            limits: adapter.limits(),
        },
        None,
    ))
    .expect("failed to create a device");

    // Configure the surface with the adapter and window size
    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = if swapchain_capabilities.formats.contains(&TextureFormat::Rgba8Unorm)
    {
        TextureFormat::Rgba8Unorm
    } 
    else if swapchain_capabilities.formats.contains(&TextureFormat::Bgra8Unorm)
    {
        TextureFormat::Bgra8Unorm
    } 
    else 
    {
        swapchain_capabilities.formats[0]
    };

    let surface_config: wgpu::SurfaceConfiguration = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: physical_size.width,
        height: physical_size.height,
        present_mode: PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: Vec::new(),
    };

    surface.configure(&device, &surface_config);

    (device, queue, Some(surface), Some(surface_config), swapchain_format)
}

fn handle_window_event(
    running: bool,
    event_loop: &mut EventLoop<()>,
    device: &wgpu::Device,
    surface: &wgpu::Surface,
    surface_config: &mut SurfaceConfiguration,
) -> bool {
    let mut running: bool = running;

    event_loop.run_return(|event, _, control_flow| {
        control_flow.set_wait();

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => running = false,
                WindowEvent::Resized(size) => {
                    surface_config.width = size.width;
                    surface_config.height = size.height;
                    surface.configure(&device, &surface_config);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    surface_config.width = new_inner_size.width;
                    surface_config.height = new_inner_size.height;
                    surface.configure(&device, &surface_config);
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        if keycode == VirtualKeyCode::Space && input.state == ElementState::Pressed {
                            println!("Spacebar pressed!");
                        }
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => control_flow.set_exit(),
            _ => (),
        }
    });

    return running;
}

fn compile_shader(shader_path: PathBuf, output_path: PathBuf) {
    if cfg!(target_os = "windows") {
        let status: std::process::ExitStatus = Command::new("./glslc.exe")
            .arg(shader_path.to_str().unwrap())
            .arg("-o")
            .arg(output_path)
            .status()
            .expect("Failed to execute glslc");
        if !status.success() {
            panic!("Shader compilation failed for {}", shader_path.to_str().unwrap());
        }
    } else {
        let status: std::process::ExitStatus = Command::new("glslc")
            .arg(shader_path.to_str().unwrap())
            .arg("-o")
            .arg(output_path)
            .status()
            .expect("Failed to execute glslc");
        if !status.success() {
            panic!("Shader compilation failed for {}", shader_path.to_str().unwrap());
        }
    }
}

fn create_render_pipeline(device: &Device, pipeline_layout: &PipelineLayout, output_format: &TextureFormat, vertex_shader: &ShaderModule, fragment_shader: &ShaderModule) -> RenderPipeline
{
    return device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &vertex_shader,
            entry_point: "main",
            buffers: &[Vertex::layout()],
        },
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &fragment_shader,
            entry_point: "main",
            targets: &[Some(ColorTargetState {
                format: *output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });
}

fn render_to_window(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &wgpu::Surface,
    surface_config: &wgpu::SurfaceConfiguration,
    render_pipeline: &wgpu::RenderPipeline,
    vbo: &wgpu::Buffer,
    bind_group: &wgpu::BindGroup,
) {
    // Get the next texture from the swapchain
    let frame = surface.get_current_texture().expect("Failed to get next swapchain texture");

    // Create a texture view for the frame
    let view = frame.texture.create_view(&TextureViewDescriptor::default());

    // Create a command encoder to record the rendering commands
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Window Render Encoder") });

    {
        // Begin a render pass to draw to the window surface
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Window Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // Set the render pipeline and bind group, then draw the vertices
        render_pass.set_pipeline(render_pipeline);
        render_pass.set_vertex_buffer(0, vbo.slice(..));
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }

    // Submit the command encoder to the queue
    queue.submit(once(encoder.finish()));

    // Present the frame to the window
    frame.present();
}

#[cfg(target_os = "linux")]
fn render_to_st7789(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_pipeline: &wgpu::RenderPipeline,
    vbo: &wgpu::Buffer,
    bind_group: &wgpu::BindGroup,
    output_image_texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    st7789: &mut raspberry_st7789_driver::RaspberryST7789Driver,
) {
    let view = output_image_texture.create_view(&TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("ST7789 Render Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ST7789 Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(render_pipeline);
        render_pass.set_vertex_buffer(0, vbo.slice(..));
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }

    queue.submit(once(encoder.finish()));

    let mut texture_data = read_texture(device, queue, output_image_texture, buffer);
    let mut retain_counter = 0;
    texture_data.retain(|_| {
        retain_counter += 1;
        retain_counter % 4 != 0
    });

    st7789.draw_raw(&texture_data, true).unwrap();
}

fn check_and_reload_shaders(
    file_watcher: &mut FileWatcher,
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    output_format: &TextureFormat,
    vertex_shader_path: &PathBuf,
    compiled_vertex_shader_path: &PathBuf,
    fragment_shader_path: &PathBuf,
    compiled_fragment_shader_path: &PathBuf,
    vertex_shader: &mut ShaderModule,
    fragment_shader: &mut ShaderModule,
    render_pipeline: &mut RenderPipeline,
) {
    if let Some(paths) = file_watcher.get_changes() {
        for path in paths {
            println!("Shader change detected: {:?}", path);
            let file_name = path.file_name().unwrap();

            if file_name == "master.vert" {
                compile_shader(vertex_shader_path.clone(), compiled_vertex_shader_path.clone());
                *vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some("master_vertex_shader"),
                    source: wgpu::util::make_spirv(&fs::read(compiled_vertex_shader_path).expect("Failed to read vertex shader")),
                });
            }

            if file_name == "master.frag" {
                compile_shader(fragment_shader_path.clone(), compiled_fragment_shader_path.clone());
                *fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: Some("master_fragment_shader"),
                    source: wgpu::util::make_spirv(&fs::read(compiled_fragment_shader_path).expect("Failed to read fragment shader")),
                });
            }

            *render_pipeline = create_render_pipeline(
                device,
                pipeline_layout,
                output_format,
                vertex_shader,
                fragment_shader,
            );
        }
    }
}

// Copies data from a texture to array of bytes
fn read_texture(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, buffer: &wgpu::Buffer) -> Vec<u8> {
    let texture_size = texture.size();
    let data_size = (texture_size.width * texture_size.height * 4) as usize; // 4 for RGBA
    // let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    //     label: Some("Read Buffer"),
    //     size: data_size as u64,
    //     usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    //     mapped_at_creation: false,
    // });

    //queue.write_buffer(&buffer, 0, &vec![0; data_size]);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Read Texture Encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * texture_size.width),
                rows_per_image: Some(texture_size.height),
            },
        },
        texture_size,
    );

    queue.submit(iter::once(encoder.finish()));
    
    // Map the buffer to read the data
    let buffer_slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();

    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        assert!(result.is_ok());
        tx.send(()).unwrap();
    });

    // Use Maintain::Poll instead of blocking the thread
    loop {
        device.poll(wgpu::Maintain::Poll);
        if rx.try_recv().is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1)); // Small sleep to reduce CPU usage
    }


    // device.poll(wgpu::Maintain::Wait);
    // rx.recv().unwrap();

    // Retrieve the data
    let data = buffer_slice.get_mapped_range();
    let mut image_data = vec![0; data.len()];
    image_data.copy_from_slice(&data);
    drop(data);

    // Unmap the buffer
    buffer.unmap();

    image_data
}

fn save_as_png(data: Vec<u8>, width: u32, height: u32, path: &str) -> Result<(), image::ImageError> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, data).unwrap();
    img.save(Path::new(path))?;
    Ok(())
}

