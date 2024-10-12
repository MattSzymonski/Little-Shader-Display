use std::{borrow::Cow, collections::HashMap, fs, iter::once, mem::{self, size_of}, path::PathBuf, process::Command, sync::LazyLock, thread, time::{Duration, SystemTime}};
use std::time::Instant;
use bytemuck::cast_slice;
use bytemuck_derive::{Pod, Zeroable};
use futures::executor::block_on;
use wgpu::{
    util::DeviceExt, BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, FragmentState, Instance, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PresentMode, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode
};
use std::path::Path;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};


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

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    texture_coordinates: [f32; 2]
}

impl Vertex {
    fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self { position: [x, y], texture_coordinates: [u, v] }
    }

    fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute { // Position
                    format: VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute { // Vertex texture coordinates
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

fn compile_shader(shader_path: &str, output_path: &str) {
    let status: std::process::ExitStatus = Command::new("./glslc.exe")
        .arg(shader_path)
        .arg("-o")
        .arg(output_path)
        .status()
        .expect("Failed to execute glslc");

    if !status.success() {
        panic!("Shader compilation failed for {}", shader_path);
    }
}

struct FileWatcher {
    path: PathBuf,
    previous_metadata: HashMap<PathBuf, SystemTime>,
}

impl FileWatcher {
    pub fn new(path: PathBuf) -> Self {
        let previous_metadata = Self::get_file_metadata(&path);
        Self { path, previous_metadata }
    }

    fn get_file_metadata(path: &Path) -> HashMap<PathBuf, SystemTime> {
        let mut file_metadata = HashMap::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                file_metadata.insert(path, modified);
                            }
                        }
                    }
                }
            }
        }

        file_metadata
    }

    fn check_for_changes(&mut self) -> Vec<PathBuf> {
        let current_metadata = Self::get_file_metadata(&self.path);
        let mut changes = Vec::new();

        // Check for modified or new files
        for (file_path, modified_time) in &current_metadata {
            match self.previous_metadata.get(file_path) {
                Some(&prev_time) if prev_time != *modified_time => {
                    changes.push(file_path.clone());
                }
                Some(_) => {
                    // Do nothing for now, you can add code here if needed
                }
                None => {
                    changes.push(file_path.clone());
                }
            }
        }

        // Check for removed files
        for file_path in self.previous_metadata.keys() {
            if !current_metadata.contains_key(file_path) {
                changes.push(file_path.clone());
            }
        }

        // Update previous metadata for the next iteration
        self.previous_metadata = current_metadata;

        changes
    }

    fn get_changes(&mut self) -> Option<Vec<PathBuf>> {
        let changes = self.check_for_changes();
        if !changes.is_empty() {
            Some(changes.clone())
        } else {
            None
        }
    }
}

static COMPILED_FRAGMENT_SHADER_PATH: &str = "./shaders/built/master.frag.spv";
static COMPILED_VERTEX_SHADER_PATH: &str = "./shaders/built/master.vert.spv";
static FRAGMENT_SHADER_PATH: &str = "./shaders/master.frag";
static VERTEX_SHADER_PATH: &str = "./shaders/master.vert";

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
    let mut frame = 0;

    let mut file_change_iterator = FileWatcher::new("D:/Programming/wgpu-cube-example/shaders".into());
    let start_time = Instant::now();
    let mut event_loop = EventLoop::new(); 

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(1280, 720))
        .with_title("Hello triangle")
        .with_visible(false)
        .build(&event_loop)
        .expect("failed to create a window");

    let physical_size = window.inner_size();

    let instance = Instance::default();
    let surface = unsafe { instance.create_surface(&window) }.expect("failed to create surface");
    let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("failed to find a suitable adapter");

    let (device, queue) = block_on(adapter.request_device(
        &DeviceDescriptor {
            label: None,
            features: Features::empty(),
            limits: adapter.limits(),
        },
        None,
    ))
    .expect("failed to create a device");

   // Create uniform buffer
   let mut uniforms = Uniforms::new();
   let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
       label: Some("Uniform Buffer"),
       contents: bytemuck::cast_slice(&[uniforms]),
       usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
   });

   // Create a bind group layout for the uniform
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

   // Create a bind group
   let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
       layout: &bind_group_layout,
       entries: &[wgpu::BindGroupEntry {
           binding: 0,
           resource: uniform_buffer.as_entire_binding(),
       }],
       label: Some("uniform_bind_group"),
   });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchian_format = if swapchain_capabilities.formats.contains(&TextureFormat::Bgra8Unorm)
    {
        TextureFormat::Bgra8Unorm
    } 
    else if swapchain_capabilities.formats.contains(&TextureFormat::Rgba8Unorm)
    {
        TextureFormat::Rgba8Unorm
    } 
    else 
    {
        swapchain_capabilities.formats[0]
    };
    
    let master_vertex_shader_bytes = include_bytes!("./shaders/built/master.vert.spv");
    let master_fragment_shader_bytes = include_bytes!("./shaders/built/master.frag.spv");

    // Create shaders
    let vertex_shader = wgpu::ShaderModuleDescriptor {
        label: Some("master_vertex_shader"),
        source: wgpu::util::make_spirv(master_vertex_shader_bytes),
    };
    let mut vertex_shader = device.create_shader_module(vertex_shader);

    let fragment_shader = wgpu::ShaderModuleDescriptor {
        label: Some("master_fragment_shader"),
        source: wgpu::util::make_spirv(master_fragment_shader_bytes),
    };
    let mut fragment_shader = device.create_shader_module(fragment_shader);

 
    // Create render pipeline
    let mut render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
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
            targets: &[Some(swapchian_format.into())],
        }),
        multiview: None,
    });

    let mut config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: swapchian_format,
        width: physical_size.width,
        height: physical_size.height,
        present_mode: PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: Vec::new(),
    };

    surface.configure(&device, &config);

    let vbo = device.create_buffer(&BufferDescriptor {
        label: None,
        size: size_of::<Vertex>() as u64 * 6,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(&vbo, 0, cast_slice(&*VERTICES));

    window.set_visible(true);
    let mut running = true;
    while running {
        frame += 1;
        event_loop.run_return(|event, _, control_flow| {
            control_flow.set_wait();

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => running = false,

                    WindowEvent::Resized(size) => {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(&device, &config);
                    }

                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        config.width = new_inner_size.width;
                        config.height = new_inner_size.height;
                        surface.configure(&device, &config);
                    }

                    _ => (),
                },

                Event::MainEventsCleared => control_flow.set_exit(),

                _ => (),
            }
        });

        // Calculate elapsed time
        let elapsed_time = start_time.elapsed().as_secs_f32();
        uniforms.time = elapsed_time;
        queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Check for shader file changes
        if let Some(paths) = file_change_iterator.get_changes() {
            for path in paths {
                println!("Shader change detected: {:?}", path);
                if (path.file_name().unwrap() == "master.vert")
                {
                    compile_shader(VERTEX_SHADER_PATH, COMPILED_VERTEX_SHADER_PATH);
                    vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("master_vertex_shader"),
                        source: wgpu::util::make_spirv(&std::fs::read(COMPILED_VERTEX_SHADER_PATH).expect("Failed to read shader file")),
                    });
                    fragment_shader = fragment_shader;
                } 

                if (path.file_name().unwrap() == "master.frag")
                {
                    compile_shader(FRAGMENT_SHADER_PATH, COMPILED_FRAGMENT_SHADER_PATH);
                    vertex_shader = vertex_shader;
                    fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("master_fragment_shader"),
                        source: wgpu::util::make_spirv(&std::fs::read(COMPILED_FRAGMENT_SHADER_PATH).expect("Failed to read shader file")),
                    });
                }    
            
                render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Hot Reloaded Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vertex_shader, // Assuming vertex shader stays the same
                        entry_point: "main",
                        buffers: &[Vertex::layout()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &fragment_shader, // The hot reloaded fragment shader
                        entry_point: "main",
                        targets: &[Some(swapchian_format.into())],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                });
            }
        }

        let frame = surface.get_current_texture().expect("failed to get next swapchain texture");
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.2,
                            g: 0.3,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&render_pipeline);
            rpass.set_vertex_buffer(0, vbo.slice(..));
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..6, 0..1); 
        }

        queue.submit(once(encoder.finish()));
        frame.present();
    }
}