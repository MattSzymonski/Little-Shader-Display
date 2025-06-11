use std::{fs, iter};
use std::iter::once;
use std::path::PathBuf;
use futures::executor::block_on;
use wgpu::util::DeviceExt;
use bytemuck_derive::{Pod, Zeroable};
use std::{
    mem::size_of,
    sync::{LazyLock},
};
use bytemuck::{cast_slice};
use std::time::Instant;

use crate::{DEBUG_OVERHEADS, SHADER_NAMES};
use crate::ST7789_OUTPUT_SIZE;
use crate::SHADERS_PATH;
use crate::COMPILED_VERTEX_SHADER_PATH;
use crate::COMPILED_FRAGMENT_SHADER_PATH;

//use crate::file_watcher::FileWatcher;
//use crate::Vertex;


// --- Data Structures for Rendering ---

// Uniform buffer struct that holds the current time to pass to the shader.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]

// Entire struct size must be a multiple of
// 16 bytes to meet GLSL buffer layout rules
struct Uniforms { 
    time: f32, // 4
    _padding_0: [f32; 3], // 12
    bluetooth_data: [f32; 3], // 12
    screen_aspect_ratio: f32, // 4 
}

impl Uniforms {
    fn new() -> Self {
        Self { time: 0.0, _padding_0: [0.0, 0.0, 0.0], bluetooth_data: [0.0, 0.0, 0.0], screen_aspect_ratio: 0.0, }
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
    // Creates a new vertex with given position and texture coordinates
    fn new(x: f32, y: f32, u: f32, v: f32) -> Self {
        Self { position: [x, y], texture_coordinates: [u, v] }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute: location 0
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // Texture coordinate attribute: location 1
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
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


pub struct Renderer {
    use_window: bool,
    use_st7789: bool,

    surface: Option<wgpu::Surface>,
    surface_config: Option<wgpu::SurfaceConfiguration>,

    #[cfg(target_os = "linux")]
    st7789_driver: Option<crate::st7789_driver::RaspberryST7789Driver>,
    st7789_render_target: Option<wgpu::Texture>,
    st7789_render_buffer: Option<wgpu::Buffer>,

    device: wgpu::Device,
    queue: wgpu::Queue,
    uniforms: Uniforms,
    vertex_shader: wgpu::ShaderModule,
    fragment_shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    output_format: wgpu::TextureFormat,
}

impl Renderer {
    pub fn new(
        use_window: bool,
        window: Option<&winit::window::Window>,
        #[cfg(target_os = "linux")]
        use_st7789: bool,
        #[cfg(target_os = "linux")]
        st7789_driver: Option<crate::st7789_driver::RaspberryST7789Driver>,
    ) -> Self {
        // --- Create GPU resources for rendering ---

        // 1. Initialize wgpu  
        let (device, queue, surface, surface_config, output_format) = match window {
            Some(window) => initialize_wgpu_with_window(window),
            None => initialize_wgpu_without_window(),
        };

        // 2. Create uniform buffer
        let uniforms = Uniforms::new();
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // 6. Compile and create shaders
        compile_shader(SHADERS_PATH.clone().join("uncompiled").join("master.vert").clone(), COMPILED_VERTEX_SHADER_PATH.clone());
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("master_vertex_shader"),
            source: wgpu::util::make_spirv(&std::fs::read(COMPILED_VERTEX_SHADER_PATH.clone()).expect("Failed to read shader file")),
        });

        compile_shader(SHADERS_PATH.clone().join("uncompiled").join(SHADER_NAMES[0]).clone(), COMPILED_FRAGMENT_SHADER_PATH.clone());
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("master_fragment_shader"),
            source: wgpu::util::make_spirv(&std::fs::read(COMPILED_FRAGMENT_SHADER_PATH.clone()).expect("Failed to read shader file")),
        });

        // 7. Create a render pipeline using the shaders
        let render_pipeline = create_render_pipeline(&device, &pipeline_layout, &output_format, &vertex_shader, &fragment_shader);

        // 8. Upload vertex buffer data
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size_of::<Vertex>() as u64 * 6,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, cast_slice(&*VERTICES));

        // 9. Create offscreen texture for rendering (used by ST7789 to read pixels)
        #[cfg(target_os = "linux")]
        let (st7789_render_target, st7789_render_buffer) = if use_st7789 {
                let output_image_size = wgpu::Extent3d {
                    width: ST7789_OUTPUT_SIZE,
                    height: ST7789_OUTPUT_SIZE,
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
        
            let data_size = (ST7789_OUTPUT_SIZE * ST7789_OUTPUT_SIZE * 4) as u64; // 4 bytes per pixel (RGBA)

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

        Self {
            use_window,
            use_st7789,
            surface,
            surface_config,
            st7789_driver,
            st7789_render_target,
            st7789_render_buffer,
            device,
            queue,
            uniforms,
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            render_pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer,
            output_format,
        }
    }

    pub fn update_uniforms(&mut self, elapsed_time: f32, bluetooth_data: String) {
        self.uniforms.time = elapsed_time;
        // Parse and assign bluetooth data into a 3-element array
        self.uniforms.bluetooth_data = if bluetooth_data.trim().is_empty() {
            [0.0, 0.0, 0.0]
        } else {
            bluetooth_data.split(',').map(|s| {
                    let v: f32 = s.split(':').nth(1).unwrap().trim().parse().unwrap();
                    (v.clamp(-10.0, 10.0)) / 10.0
                }).collect::<Vec<_>>().try_into().unwrap()
        };
        // Assign screen aspect ratio, calculate it if rendering to window
        self.uniforms.screen_aspect_ratio = if self.use_window {
            self.surface_config.as_ref().unwrap().width as f32 / self.surface_config.as_ref().unwrap().height as f32
        } else {
            1.0
        };

        // Write updated uniforms to the uniform buffer
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
    }

    pub fn recompile_shaders(
        &mut self,
        shader_index: usize,
        recompile_vertex_shader: bool,
        recompile_fragment_shader: bool,
    ) {
        if recompile_vertex_shader {
            compile_shader(
                SHADERS_PATH.join("uncompiled").join("master.vert").clone(),
                COMPILED_VERTEX_SHADER_PATH.clone(),
            );
            self.vertex_shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("vertex_shader"),
                source: wgpu::util::make_spirv(&fs::read(COMPILED_VERTEX_SHADER_PATH.clone()).expect("Failed to read vertex shader")),
            });
        }

        if recompile_fragment_shader {
            compile_shader(
                SHADERS_PATH.join("uncompiled").join(SHADER_NAMES[shader_index]).clone(),
                COMPILED_FRAGMENT_SHADER_PATH.clone(),
            );
            self.fragment_shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fragment_shader"),
                source: wgpu::util::make_spirv(&fs::read(COMPILED_FRAGMENT_SHADER_PATH.clone()).expect("Failed to read fragment shader")),
            });
        }

        self.render_pipeline = create_render_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.output_format,
            &self.vertex_shader,
            &self.fragment_shader,
        );
    }   

    pub fn render(
        &mut self
    ) {
        if self.use_window {
            // Render to the window if enabled
            self.render_to_window();
        }

        #[cfg(target_os = "linux")]
        if self.use_st7789 {
            // Render to the ST7789 display if enabled
            self.render_to_st7789();
        }
    }

    fn render_to_window(
        &self,
    ) {
        // Get the next texture from the swapchain
        let frame = self.surface.as_ref().unwrap().get_current_texture().expect("Failed to get next swapchain texture");

        // Create a texture view for the frame
        let texture_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder to record the rendering commands
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Window Render Encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

           // Set the render pipeline and bind group, then draw the vertices
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // Submit the command encoder to the queue
        self.queue.submit(once(encoder.finish()));

        // Present the frame to the window
        frame.present();
    }

    fn render_to_st7789(
        &mut self,
    ) {
        let render_start = Instant::now();

        // Create a texture view for the frame
        let texture_view = self.st7789_render_target.as_mut().unwrap().create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder to record the rendering commands
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Window Render Encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

           // Set the render pipeline and bind group, then draw the vertices
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // Submit the command encoder to the queue
        self.queue.submit(once(encoder.finish()));

        if DEBUG_OVERHEADS {
            self.device.poll(wgpu::Maintain::Wait); // Wait for GPU to finish
        }
        let render_ms = render_start.elapsed().as_secs_f64() * 1000.0;

        // Present the frame to the window
        let texture_data = self.read_texture(
            self.st7789_render_target.as_ref().expect("st7789_render_target is None"),
            self.st7789_render_buffer.as_ref().expect("st7789_render_buffer is None"),
        );
        let readback_ms = render_start.elapsed().as_secs_f64() * 1000.0 - render_ms;

        // Convert RGBA8888 to RGB565 (LE packed bytes)
        let rgb565_bytes = rgba8888_to_rgb565_u8(&texture_data, false);
        let color_conversion_ms = render_start.elapsed().as_secs_f64() * 1000.0 - render_ms - readback_ms;

        self.st7789_driver.as_mut().unwrap().draw(&rgb565_bytes).unwrap();
        let draw_ms = render_start.elapsed().as_secs_f64() * 1000.0 - render_ms - readback_ms - color_conversion_ms;

        if DEBUG_OVERHEADS {
            println!("Render time: {:.2}ms, GPU readback time: {:.2}ms, Color conversion time: {:.2}ms, Draw time: {:.2}ms", render_ms, readback_ms, color_conversion_ms, draw_ms);
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(surface_config) = &mut self.surface_config {
            surface_config.width = width;
            surface_config.height = height;
            self.surface.as_ref().unwrap().configure(&self.device, surface_config);
        }
    }

    // Copies data from a texture to array of bytes
    fn read_texture(&self, texture: &wgpu::Texture, buffer: &wgpu::Buffer) -> Vec<u8> {
        let texture_size = texture.size();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

        self.queue.submit(iter::once(encoder.finish()));
        
        // Map the buffer to read the data
        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            assert!(result.is_ok());
            tx.send(()).unwrap();
        });

        // Use Maintain::Poll instead of blocking the thread
        loop {
            self.device.poll(wgpu::Maintain::Poll);
            if rx.try_recv().is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1)); // Small sleep to reduce CPU usage
        }

        // Retrieve the data
        let data = buffer_slice.get_mapped_range();
        let mut image_data = vec![0; data.len()];
        image_data.copy_from_slice(&data);
        drop(data);

        // Unmap the buffer
        buffer.unmap();

        image_data
    }
}

// Compiles GLSL shaders to SPIR-V using glslc or glslc.exe
fn compile_shader(shader_path: PathBuf, output_path: PathBuf) {
    println!("Compiling shader: {}", shader_path.display());

    let compiler = if cfg!(target_os = "windows") {
        "./glslc.exe"
    } else {
        "glslc"
    };

    let status = std::process::Command::new(compiler)
        .arg(shader_path.to_str().unwrap())
        .arg("-o")
        .arg(output_path)
        .status()
        .expect("Failed to execute shader compiler");

    if !status.success() {
        panic!("Shader compilation failed: {}", shader_path.display());
    }
}

// Helper to create a render pipeline
fn create_render_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    output_format: &wgpu::TextureFormat,
    vertex_shader: &wgpu::ShaderModule,
    fragment_shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: vertex_shader,
            entry_point: "main",
            buffers: &[Vertex::layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: fragment_shader,
            entry_point: "main",
            targets: &[Some(wgpu::ColorTargetState {
                format: *output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn initialize_wgpu_without_window() -> (wgpu::Device, wgpu::Queue, Option<wgpu::Surface>, Option<wgpu::SurfaceConfiguration>, wgpu::TextureFormat) {
    
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

    (device, queue, None, None, wgpu::TextureFormat::Rgba8Unorm)
}

fn initialize_wgpu_with_window(window: &winit::window::Window) -> (wgpu::Device, wgpu::Queue, Option<wgpu::Surface>, Option<wgpu::SurfaceConfiguration>, wgpu::TextureFormat) {

    // Get the physical size of the window
    let physical_size = window.inner_size();

    // Create a wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

    // Create a surface for the window
    let surface = unsafe { instance.create_surface(&window) }.expect("failed to create surface");

    // Create addapter with the surface
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("failed to find a suitable adapter");
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
    .expect("failed to create a device");

    // Configure the surface with the adapter and window size
    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = wgpu::TextureFormat::Bgra8Unorm;

    // Create a surface configuration with the selected format and window size
    let surface_config: wgpu::SurfaceConfiguration = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: physical_size.width,
        height: physical_size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: Vec::new(),
    };

    // Apply the surface configuration to the surface
    surface.configure(&device, &surface_config);

    // Return the device, queue, surface, surface configuration, and swapchain format
    (device, queue, Some(surface), Some(surface_config), swapchain_format)
}

fn save_as_png(data: Vec<u8>, width: u32, height: u32, path: &str) -> Result<(), image::ImageError> {
    let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> = image::ImageBuffer::from_raw(width, height, data).unwrap();
    img.save(std::path::Path::new(path))?;
    Ok(())
}

// Converts RGBA8888 (4 bytes per pixel) to RGB565 (2 bytes per pixel, little-endian)
// Skips the alpha channel entirely.
fn rgba8888_to_rgb565_u8(input: &[u8], flip_order: bool) -> Vec<u8> {
    let mut output = Vec::with_capacity((input.len() / 4) * 2); // 2 bytes per pixel (RGB565)
    for chunk in input.chunks_exact(4) {

        let r = if flip_order { chunk[2] } else { chunk[0] };
        let g = chunk[1];
        let b = if flip_order { chunk[0] } else { chunk[2] };

        // Convert RGBA8888 to RGB565
        let rgb565: u16 =
            ((r as u16 & 0xF8) << 8) | // Red: upper 5 bits
            ((g as u16 & 0xFC) << 3) | // Green: upper 6 bits
            ((b as u16) >> 3);         // Blue: upper 5 bits

        // Split color value into two consecutive bytes 
        output.push((rgb565 & 0xFF) as u8);      // Low byte
        output.push((rgb565 >> 8) as u8);        // High byte
    }

    output
}

fn rgba8888_to_rgb565(input: &[u8], flip_order: bool) -> Vec<u16> {
    let mut output = Vec::with_capacity((input.len() / 4) * 2); // 2 bytes per pixel (RGB565)
    for chunk in input.chunks_exact(4) {

        let r = if flip_order { chunk[2] } else { chunk[0] };
        let g = chunk[1];
        let b = if flip_order { chunk[0] } else { chunk[2] };

        // Convert RGBA8888 to RGB565
        let rgb565: u16 =
            ((r as u16 & 0xF8) << 8) | // Red: upper 5 bits
            ((g as u16 & 0xFC) << 3) | // Green: upper 6 bits
            ((b as u16) >> 3);         // Blue: upper 5 bits

        // Split color value into two consecutive bytes 
        output.push(rgb565);      // Low byte
    }

    output
}