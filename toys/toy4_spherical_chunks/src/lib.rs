// Shared library for toy4 spherical chunk streaming

pub const PLANET_RADIUS: f32 = 1_000_000.0; // 1000km radius
pub const DEFAULT_ALTITUDE: f32 = 30.0; // 30m above surface (tuned for visual density)
pub const DEFAULT_SPEED: f32 = 100.0; // 100 m/s tangential velocity

// === Data Structures ===

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub _padding1: f32,
    pub uv: [f32; 2],
    pub _padding2: [f32; 2],
    pub normal: [f32; 3],
    pub _padding3: f32,
    pub grid_coord: [f32; 2], // World-space grid coordinates (in meters)
    pub _padding4: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereParams {
    pub planet_radius: f32,
    pub chunk_origin_lon_cell: i32, // Global grid cell X coordinate
    pub chunk_origin_lat_cell: i32, // Global grid cell Z coordinate
    pub grid_size: u32,
    pub grid_spacing: f32,
    pub base_amplitude: f32,   // Height variation (meters)
    pub base_frequency: f32,   // Noise scale
    pub detail_amplitude: f32, // Detail layer height
    pub detail_frequency: f32, // Detail layer scale
    pub _padding1: f32,
    pub _padding2: f32,
    pub _padding3: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub debug_chunk_boundaries: u32, // 0 = off, 1 = on
}

// === Chunk System ===

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct ChunkId {
    pub lat_cell: i32,
    pub lon_cell: i32,
}

impl ChunkId {
    pub fn from_camera_angle(camera_angle: f32, chunk_angular_size: f32) -> Self {
        // Camera is on equator (lat = 0)
        ChunkId {
            lat_cell: 0,
            lon_cell: (camera_angle / chunk_angular_size).floor() as i32,
        }
    }

    pub fn center_lon(&self, chunk_angular_size: f32) -> f32 {
        (self.lon_cell as f32 + 0.5) * chunk_angular_size
    }

    pub fn neighbors(&self) -> Vec<ChunkId> {
        let mut neighbors = Vec::new();
        // 3×3 grid (sufficient for 200m fog distance)
        for dlat in -1..=1 {
            for dlon in -1..=1 {
                neighbors.push(ChunkId {
                    lat_cell: self.lat_cell + dlat,
                    lon_cell: self.lon_cell + dlon,
                });
            }
        }
        neighbors
    }
}

pub struct Chunk {
    pub id: ChunkId,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl Chunk {
    pub fn create(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        id: ChunkId,
        chunk_size: u32,
        grid_spacing: f32,
        _chunk_angular_size: f32,
    ) -> Self {
        let vertex_count = chunk_size * chunk_size;

        // Create vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Vertex Buffer"),
            size: (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Calculate global integer grid origin for this chunk
        // Each chunk occupies (chunk_size - 1) grid cells in each dimension
        // chunk_size = 256 means 255 cells (0-255 vertices = 255 cells)
        let cells_per_chunk = (chunk_size - 1) as i32;
        let chunk_origin_lon_cell = id.lon_cell * cells_per_chunk;
        let chunk_origin_lat_cell = id.lat_cell * cells_per_chunk;

        // Create sphere params for this chunk
        let sphere_params = SphereParams {
            planet_radius: PLANET_RADIUS,
            chunk_origin_lon_cell,
            chunk_origin_lat_cell,
            grid_size: chunk_size,
            grid_spacing,
            base_amplitude: 10.0,      // 10m height variation
            base_frequency: 13333.0,   // 75m hill spacing (1.0 / (75m / planet_radius))
            detail_amplitude: 3.0,     // 3m detail variation
            detail_frequency: 50000.0, // 20m detail spacing (1.0 / (20m / planet_radius))
            _padding1: 0.0,
            _padding2: 0.0,
            _padding3: 0.0,
        };

        let sphere_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Params Buffer"),
            size: std::mem::size_of::<SphereParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&sphere_params_buffer, 0, bytemuck::bytes_of(&sphere_params));

        // Create compute bind group
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chunk Compute Bind Group"),
            layout: compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sphere_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Run compute shader once to generate chunk geometry
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Chunk Compute Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Chunk Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(compute_pipeline);
            compute_pass.set_bind_group(0, &compute_bind_group, &[]);
            let workgroup_count = (vertex_count + 255) / 256;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Create index buffer
        let indices = generate_grid_indices(chunk_size);
        let index_count = indices.len() as u32;

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Index Buffer"),
            size: (indices.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

        Chunk {
            id,
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }
}

// === Camera System ===

pub struct OrbitCamera {
    pub altitude: f32,
    pub angular_pos: f32,
    pub angular_velocity: f32,
    pub time: f32, // For lateral oscillation
}

impl OrbitCamera {
    pub fn new(altitude: f32, speed_m_s: f32) -> Self {
        let r = PLANET_RADIUS + altitude;
        let angular_velocity = speed_m_s / r;

        Self {
            altitude,
            angular_pos: 0.0,
            angular_velocity,
            time: 0.0,
        }
    }

    pub fn at_angle(altitude: f32, angle: f32) -> Self {
        Self {
            altitude,
            angular_pos: angle,
            angular_velocity: 0.0,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.angular_pos += self.angular_velocity * dt;
        self.time += dt;
    }

    pub fn position(&self) -> glam::Vec3 {
        let r = PLANET_RADIUS + self.altitude;

        // Add lateral oscillation: +/- 50m sinusoidal movement
        let oscillation_amplitude = 50.0; // meters
        let oscillation_frequency = 0.3; // Hz (one cycle every ~3 seconds)
        let lateral_offset = (self.time * oscillation_frequency * 2.0 * std::f32::consts::PI).sin()
            * oscillation_amplitude;
        let lat_offset = lateral_offset / PLANET_RADIUS;

        let lat = lat_offset;
        let lon = self.angular_pos;

        glam::Vec3::new(
            r * lat.cos() * lon.cos(),
            r * lat.sin(),
            r * lat.cos() * lon.sin(),
        )
    }

    pub fn view_proj_matrix(&self, aspect_ratio: f32) -> ([[f32; 4]; 4], glam::Vec3) {
        let pos = self.position();

        // Look ahead along orbital path
        let look_ahead_meters = 300.0;
        let look_ahead_angle = self.angular_pos + look_ahead_meters / PLANET_RADIUS;

        let look_at = glam::Vec3::new(
            PLANET_RADIUS * look_ahead_angle.cos(),
            0.0,
            PLANET_RADIUS * look_ahead_angle.sin(),
        );

        // Radial up vector (points away from planet center)
        let up = pos.normalize();

        let view = glam::Mat4::look_at_rh(pos, look_at, up);
        let proj =
            glam::Mat4::perspective_rh(60.0_f32.to_radians(), aspect_ratio, 1.0, 2_000_000.0);

        ((proj * view).to_cols_array_2d(), pos)
    }

    pub fn camera_uniforms(
        &self,
        aspect_ratio: f32,
        debug_chunk_boundaries: bool,
    ) -> CameraUniforms {
        let (view_proj, pos) = self.view_proj_matrix(aspect_ratio);
        CameraUniforms {
            view_proj,
            camera_pos: [pos.x, pos.y, pos.z],
            debug_chunk_boundaries: if debug_chunk_boundaries { 1 } else { 0 },
        }
    }
}

// === Rendering Helpers ===

pub fn create_camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Camera Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Render Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("sphere_render.wgsl").into()),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                        format: wgpu::VertexFormat::Float32x3, // position
                    },
                    wgpu::VertexAttribute {
                        offset: 16,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x2, // uv
                    },
                    wgpu::VertexAttribute {
                        offset: 32,
                        shader_location: 2,
                        format: wgpu::VertexFormat::Float32x3, // normal
                    },
                    wgpu::VertexAttribute {
                        offset: 48,
                        shader_location: 3,
                        format: wgpu::VertexFormat::Float32x2, // grid_coord
                    },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
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
    })
}

// === Helper Functions ===

pub fn generate_grid_indices(grid_size: u32) -> Vec<u32> {
    let mut indices = Vec::new();

    // Generate line indices for wireframe grid
    // Only draw interior lines to avoid double-drawing at chunk boundaries

    // Horizontal lines (skip top edge z=0 to avoid overlap with neighbor)
    for z in 1..grid_size {
        for x in 0..grid_size - 1 {
            let current = z * grid_size + x;
            let next = current + 1;
            indices.push(current);
            indices.push(next);
        }
    }

    // Vertical lines (skip left edge x=0 to avoid overlap with neighbor)
    for x in 1..grid_size {
        for z in 0..grid_size - 1 {
            let current = z * grid_size + x;
            let next = (z + 1) * grid_size + x;
            indices.push(current);
            indices.push(next);
        }
    }

    indices
}
