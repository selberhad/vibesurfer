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
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereParams {
    pub planet_radius: f32,
    pub chunk_center_lat: f32,
    pub chunk_center_lon: f32,
    pub grid_size: u32,
    pub grid_spacing: f32,
    pub _padding1: f32,
    pub _padding2: f32,
    pub _padding3: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _padding: f32,
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
        chunk_angular_size: f32,
    ) -> Self {
        let vertex_count = chunk_size * chunk_size;

        // Create vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Vertex Buffer"),
            size: (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Create sphere params for this chunk
        let sphere_params = SphereParams {
            planet_radius: PLANET_RADIUS,
            chunk_center_lat: 0.0,
            chunk_center_lon: id.center_lon(chunk_angular_size),
            grid_size: chunk_size,
            grid_spacing,
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
}

impl OrbitCamera {
    pub fn new(altitude: f32, speed_m_s: f32) -> Self {
        let r = PLANET_RADIUS + altitude;
        let angular_velocity = speed_m_s / r;

        Self {
            altitude,
            angular_pos: 0.0,
            angular_velocity,
        }
    }

    pub fn at_angle(altitude: f32, angle: f32) -> Self {
        Self {
            altitude,
            angular_pos: angle,
            angular_velocity: 0.0,
        }
    }

    pub fn position(&self) -> glam::Vec3 {
        let r = PLANET_RADIUS + self.altitude;
        let lat = 0.0_f32;
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

    pub fn camera_uniforms(&self, aspect_ratio: f32) -> CameraUniforms {
        let (view_proj, pos) = self.view_proj_matrix(aspect_ratio);
        CameraUniforms {
            view_proj,
            camera_pos: [pos.x, pos.y, pos.z],
            _padding: 0.0,
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
                        format: wgpu::VertexFormat::Float32x3,
                    },
                    wgpu::VertexAttribute {
                        offset: 16,
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

    for z in 0..grid_size - 1 {
        for x in 0..grid_size - 1 {
            let top_left = z * grid_size + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * grid_size + x;
            let bottom_right = bottom_left + 1;

            // Two triangles per quad (as lines)
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(bottom_left);
            indices.push(bottom_right);
            indices.push(bottom_right);
            indices.push(top_right);
            indices.push(top_right);
            indices.push(top_left);
        }
    }

    indices
}
