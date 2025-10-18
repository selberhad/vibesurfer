use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toy3_infinite_camera::{
    create_perspective_view_proj_matrix, generate_grid_indices, CameraState, CameraUniforms,
    TerrainParams, Vertex,
};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// === FPS Tracker ===

struct FpsTracker {
    frame_times: VecDeque<Duration>,
    last_frame: Instant,
    last_print: Instant,
    min_fps: f32,
    max_fps: f32,
}

impl FpsTracker {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_times: VecDeque::new(),
            last_frame: now,
            last_print: now,
            min_fps: f32::MAX,
            max_fps: 0.0,
        }
    }

    fn record_frame(&mut self) {
        let now = Instant::now();
        let frame_time = now - self.last_frame;
        self.last_frame = now;

        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        // Track min/max FPS
        let current_fps = self.current_fps();
        if current_fps > 0.0 {
            self.min_fps = self.min_fps.min(current_fps);
            self.max_fps = self.max_fps.max(current_fps);
        }

        // Print FPS every second
        if now - self.last_print > Duration::from_secs(1) {
            let (min, avg, max) = self.stats();
            println!("FPS - Min: {:.1}, Avg: {:.1}, Max: {:.1}", min, avg, max);
            self.last_print = now;
        }
    }

    fn current_fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }

        let total: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total.as_secs_f32() / self.frame_times.len() as f32;

        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    fn stats(&self) -> (f32, f32, f32) {
        (self.min_fps, self.current_fps(), self.max_fps)
    }
}

// === Main App ===

struct App {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // Terrain compute resources
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    terrain_params_buffer: wgpu::Buffer,
    grid_size: u32,
    vertex_count: u32,

    // Rendering resources
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    fps_tracker: FpsTracker,
    start_time: Instant,
    camera: CameraState,
    window: Arc<Window>,
}

impl App {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let grid_size = 512u32; // Medium scale: 262,144 vertices
        let vertex_count = grid_size * grid_size;

        // Initialize wgpu
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // === Create Compute Pipeline ===

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain_compute.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    // Vertex buffer (storage, read-write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Terrain params (uniform)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Terrain Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        // === Create Buffers ===

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let grid_spacing = 2.0; // 2m between vertices
        let terrain_params = TerrainParams::new(grid_size, grid_spacing, [0.0, 0.0, 0.0], 0.0);

        let terrain_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Terrain Params Buffer"),
            size: std::mem::size_of::<TerrainParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(
            &terrain_params_buffer,
            0,
            bytemuck::bytes_of(&terrain_params),
        );

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: terrain_params_buffer.as_entire_binding(),
                },
            ],
        });

        // === Create Render Pipeline ===

        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("terrain_render.wgsl").into()),
        });

        // Camera uniforms
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 16, // After position (12 bytes) + padding1 (4 bytes)
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Initialize camera (perspective view)
        let aspect = size.width as f32 / size.height as f32;
        let view_proj = create_perspective_view_proj_matrix(aspect);
        queue.write_buffer(
            &camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniforms { view_proj }),
        );

        // Generate index buffer for wireframe triangles
        let indices = generate_grid_indices(grid_size);
        let index_count = indices.len() as u32;
        println!("Grid size: {}, Total indices: {}", grid_size, index_count);
        println!("First 20 indices: {:?}", &indices[..20.min(indices.len())]);

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (index_count as u64) * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: true,
        });

        {
            let mut buffer_view = index_buffer.slice(..).get_mapped_range_mut();
            buffer_view.copy_from_slice(bytemuck::cast_slice(&indices));
        }
        index_buffer.unmap();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            compute_pipeline,
            compute_bind_group,
            vertex_buffer,
            terrain_params_buffer,
            grid_size,
            vertex_count,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            index_buffer,
            index_count,
            fps_tracker: FpsTracker::new(),
            start_time: Instant::now(),
            camera: CameraState::new([0.0, 0.0, 0.0], [0.0, 0.0, 10.0]),
            window,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Calculate elapsed time
        let time = self.start_time.elapsed().as_secs_f32();

        // Simulate audio bands (sine waves)
        let audio_low = 5.0 + 5.0 * (time * 0.5).sin();
        let audio_mid = 3.0 + 2.0 * (time * 1.0).sin();
        let _audio_high = 2.0 + 1.0 * (time * 2.0).sin();

        // Update camera position based on velocity
        self.camera.update();

        // Update camera matrix (only needs to update on window resize, but doing each frame is fine)
        let aspect = self.size.width as f32 / self.size.height as f32;
        let view_proj = create_perspective_view_proj_matrix(aspect);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&CameraUniforms { view_proj }),
        );

        // Update terrain parameters with audio modulation
        let mut terrain_params =
            TerrainParams::new(self.grid_size, 2.0, self.camera.position, time);
        terrain_params.with_audio(audio_low, audio_mid);

        self.queue.write_buffer(
            &self.terrain_params_buffer,
            0,
            bytemuck::bytes_of(&terrain_params),
        );

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // === Compute Pass: Generate Terrain ===
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Terrain Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

            let workgroup_count = (self.vertex_count + 255) / 256;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        // === Render Pass: Draw Wireframe ===
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.fps_tracker.record_frame();

        Ok(())
    }
}

// === Application Handler ===

struct AppState {
    app: Option<App>,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Toy 3: Toroidal Camera Navigation")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280u32, 720u32));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let app = pollster::block_on(App::new(window));
        self.app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                if let Some(app) = &mut self.app {
                    app.resize(physical_size);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(app) = &mut self.app {
                    match app.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => app.resize(app.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(app) = &self.app {
            app.window.request_redraw();
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app_state = AppState { app: None };
    event_loop.run_app(&mut app_state).unwrap();
}
