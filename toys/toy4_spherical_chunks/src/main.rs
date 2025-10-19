use std::collections::HashMap;
use std::sync::Arc;
use toy4_spherical_chunks::*;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// === Configuration ===

const DEFAULT_CHUNK_SIZE: u32 = 256; // 256x256 vertices per chunk
const DEFAULT_ALTITUDE: f32 = 100.0; // 100m above surface
const DEFAULT_SPEED: f32 = 100.0; // 100 m/s tangential velocity
const DEFAULT_SPACING: f32 = 2.0; // 2m between vertices

// === Orbital Camera ===

struct OrbitCamera {
    altitude: f32,         // Height above surface (m)
    angular_pos: f32,      // Current angle around sphere (radians)
    angular_velocity: f32, // Orbital speed (rad/s)
    paused: bool,
}

impl OrbitCamera {
    fn new(altitude: f32, speed_m_s: f32) -> Self {
        // Convert linear speed to angular velocity: v = r * ω
        let r = PLANET_RADIUS + altitude;
        let angular_velocity = speed_m_s / r;

        Self {
            altitude,
            angular_pos: 0.0,
            angular_velocity,
            paused: false,
        }
    }

    fn update(&mut self, dt: f32) {
        if !self.paused {
            self.angular_pos += self.angular_velocity * dt;
            self.angular_pos %= std::f32::consts::TAU; // Wrap at 2π
        }
    }

    fn position(&self) -> glam::Vec3 {
        // Equatorial orbit (latitude = 0)
        let r = PLANET_RADIUS + self.altitude;
        let lat: f32 = 0.0;
        let lon = self.angular_pos;

        // Spherical to Cartesian
        glam::Vec3::new(
            r * lat.cos() * lon.cos(),
            r * lat.sin(),
            r * lat.cos() * lon.sin(),
        )
    }

    fn adjust_altitude(&mut self, delta: f32) {
        self.altitude = (self.altitude + delta).max(1.0); // Min 1m altitude
                                                          // Update angular velocity to maintain same linear speed
        let linear_speed = self.angular_velocity * (PLANET_RADIUS + self.altitude - delta);
        self.angular_velocity = linear_speed / (PLANET_RADIUS + self.altitude);
        println!(
            "Altitude: {:.1}m (radius: {:.1}m)",
            self.altitude,
            PLANET_RADIUS + self.altitude
        );
    }

    fn adjust_speed(&mut self, delta_m_s: f32) {
        let current_speed = self.angular_velocity * (PLANET_RADIUS + self.altitude);
        let new_speed = (current_speed + delta_m_s).max(1.0);
        self.angular_velocity = new_speed / (PLANET_RADIUS + self.altitude);
        println!("Orbital speed: {:.1} m/s", new_speed);
    }

    fn view_proj_matrix(&self, aspect_ratio: f32) -> [[f32; 4]; 4] {
        let pos = self.position();

        // Look at chunk center at (PLANET_RADIUS, 0, 0)
        let chunk_center = glam::Vec3::new(PLANET_RADIUS, 0.0, 0.0);

        let view = glam::Mat4::look_at_rh(pos, chunk_center, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(
            60.0_f32.to_radians(),
            aspect_ratio,
            1.0,
            2_000_000.0, // Far plane beyond planet radius
        );

        (proj * view).to_cols_array_2d()
    }
}

// === Main App ===

struct App {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // Compute resources
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group_layout: wgpu::BindGroupLayout,

    // Render resources
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // Chunk management
    chunks: HashMap<ChunkId, Chunk>,
    chunk_size: u32,
    chunk_angular_size: f32,
    grid_spacing: f32,

    camera: OrbitCamera,
    last_frame: std::time::Instant,
    frame_count: u32,
    fps_timer: std::time::Instant,
    window: Arc<Window>,
}

impl App {
    async fn new(window: Arc<Window>, chunk_size: u32) -> Self {
        let size = window.inner_size();

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
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
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

        // Create compute pipeline
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sphere_compute.wgsl").into()),
        });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
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
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        // Create camera buffer
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create render pipeline
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sphere_render.wgsl").into()),
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
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 16, // After position + padding
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Calculate chunk angular size
        let grid_spacing = DEFAULT_SPACING;
        let chunk_extent_meters = chunk_size as f32 * grid_spacing;
        let chunk_angular_size = chunk_extent_meters / PLANET_RADIUS;

        // Create initial chunk at camera position
        let camera = OrbitCamera::new(DEFAULT_ALTITUDE, DEFAULT_SPEED);
        let camera_chunk_id = ChunkId::from_camera_angle(camera.angular_pos, chunk_angular_size);

        let chunk = Chunk::create(
            &device,
            &queue,
            &compute_pipeline,
            &compute_bind_group_layout,
            camera_chunk_id,
            chunk_size,
            grid_spacing,
            chunk_angular_size,
        );

        let mut chunks = HashMap::new();
        chunks.insert(camera_chunk_id, chunk);

        println!("Chunk angular size: {:.6} radians", chunk_angular_size);
        println!("Initial chunk: {:?}", camera_chunk_id);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            compute_pipeline,
            compute_bind_group_layout,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            chunks,
            chunk_size,
            chunk_angular_size,
            grid_spacing,
            camera,
            last_frame: std::time::Instant::now(),
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
            window,
        }
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.camera.update(dt);

        // Update chunk streaming (3×3 grid)
        self.update_chunks();

        // FPS counter
        self.frame_count += 1;
        if (now - self.fps_timer).as_secs() >= 1 {
            let total_vertices: u32 = self.chunks.len() as u32 * self.chunk_size * self.chunk_size;
            println!(
                "FPS: {} | Chunks: {} | Vertices: {} | Spacing: {:.2}m",
                self.frame_count,
                self.chunks.len(),
                total_vertices,
                self.grid_spacing
            );
            self.frame_count = 0;
            self.fps_timer = now;
        }
    }

    fn update_chunks(&mut self) {
        use std::collections::HashSet;

        // Determine which chunk camera is in
        let center_chunk_id =
            ChunkId::from_camera_angle(self.camera.angular_pos, self.chunk_angular_size);

        // Get 3×3 grid of chunks around camera
        let needed_chunks: HashSet<ChunkId> = center_chunk_id.neighbors().into_iter().collect();

        // Unload chunks that are too far away
        self.chunks.retain(|id, _| {
            let keep = needed_chunks.contains(id);
            if !keep {
                println!("Unloaded chunk {:?}", id);
            }
            keep
        });

        // Load missing chunks
        for chunk_id in needed_chunks {
            if !self.chunks.contains_key(&chunk_id) {
                let chunk = Chunk::create(
                    &self.device,
                    &self.queue,
                    &self.compute_pipeline,
                    &self.compute_bind_group_layout,
                    chunk_id,
                    self.chunk_size,
                    self.grid_spacing,
                    self.chunk_angular_size,
                );
                self.chunks.insert(chunk_id, chunk);
                println!("Loaded chunk {:?}", chunk_id);
            }
        }
    }

    fn render(&mut self) {
        // Update camera uniform
        let aspect_ratio = self.size.width as f32 / self.size.height as f32;
        let view_proj = self.camera.view_proj_matrix(aspect_ratio);
        let camera_uniforms = CameraUniforms { view_proj };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniforms));

        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            // Render all chunks
            for chunk in self.chunks.values() {
                render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..chunk.index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn handle_input(&mut self, keycode: KeyCode) {
        match keycode {
            KeyCode::Digit1 => self.camera.adjust_altitude(10.0),
            KeyCode::Digit2 => self.camera.adjust_altitude(-10.0),
            KeyCode::Digit3 => self.camera.adjust_speed(10.0),
            KeyCode::Digit4 => self.camera.adjust_speed(-10.0),
            KeyCode::Digit5 => {
                self.grid_spacing = (self.grid_spacing * 0.5).max(0.25);
                println!("Grid spacing: {:.2}m", self.grid_spacing);
                self.recreate_chunks();
            }
            KeyCode::Digit6 => {
                self.grid_spacing = (self.grid_spacing * 2.0).min(10.0);
                println!("Grid spacing: {:.2}m", self.grid_spacing);
                self.recreate_chunks();
            }
            KeyCode::Space => {
                self.camera.paused = !self.camera.paused;
                println!(
                    "Orbit {}",
                    if self.camera.paused {
                        "paused"
                    } else {
                        "resumed"
                    }
                );
            }
            KeyCode::KeyP => {
                let pos = self.camera.position();
                println!(
                    "Camera: altitude={:.1}m, pos=[{:.1}, {:.1}, {:.1}], angle={:.3}rad",
                    self.camera.altitude, pos.x, pos.y, pos.z, self.camera.angular_pos
                );
            }
            _ => {}
        }
    }

    fn recreate_chunks(&mut self) {
        // Recalculate chunk angular size
        let chunk_extent_meters = self.chunk_size as f32 * self.grid_spacing;
        self.chunk_angular_size = chunk_extent_meters / PLANET_RADIUS;

        // Clear existing chunks
        self.chunks.clear();

        // Create chunk at current camera position
        let camera_chunk_id =
            ChunkId::from_camera_angle(self.camera.angular_pos, self.chunk_angular_size);
        let chunk = Chunk::create(
            &self.device,
            &self.queue,
            &self.compute_pipeline,
            &self.compute_bind_group_layout,
            camera_chunk_id,
            self.chunk_size,
            self.grid_spacing,
            self.chunk_angular_size,
        );
        self.chunks.insert(camera_chunk_id, chunk);
        println!("Recreated chunk {:?}", camera_chunk_id);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

// === Event Handler ===

struct AppHandler {
    app: Option<App>,
    chunk_size: u32,
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Toy 4: Spherical Chunk Streaming")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));

        let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
        let app = pollster::block_on(App::new(window, self.chunk_size));
        self.app = Some(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(app) = &mut self.app else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                app.resize(physical_size);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                app.handle_input(keycode);
            }
            WindowEvent::RedrawRequested => {
                app.update();
                app.render();
                app.window.request_redraw();
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

// === Main ===

fn main() {
    env_logger::init();

    // Parse command-line args
    let args: Vec<String> = std::env::args().collect();
    let chunk_size = if args.len() > 1 {
        args[1].parse().unwrap_or(DEFAULT_CHUNK_SIZE)
    } else {
        DEFAULT_CHUNK_SIZE
    };

    println!("=== Toy 4: Spherical Chunk Streaming ===");
    println!(
        "Planet radius: {}m ({:.1}km)",
        PLANET_RADIUS,
        PLANET_RADIUS / 1000.0
    );
    println!("Chunk size: {}x{} vertices", chunk_size, chunk_size);
    println!("\nControls:");
    println!("  1/2 - Adjust altitude");
    println!("  3/4 - Adjust speed");
    println!("  5/6 - Adjust grid spacing");
    println!("  Space - Pause/resume orbit");
    println!("  P - Print camera stats");
    println!();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut handler = AppHandler {
        app: None,
        chunk_size,
    };

    event_loop.run_app(&mut handler).unwrap();
}
