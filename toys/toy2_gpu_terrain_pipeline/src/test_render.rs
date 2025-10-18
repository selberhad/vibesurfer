// Headless rendering test for debugging wireframe
use bytemuck::{Pod, Zeroable};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const GRID_SIZE: u32 = 10;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    _padding1: f32,
    uv: [f32; 2],
    _padding2: [f32; 2], // Pad to 32 bytes for WGSL array alignment
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct TerrainParams {
    base_amplitude: f32,
    base_frequency: f32,
    grid_size: u32,
    grid_spacing: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CameraUniforms {
    view_proj: [[f32; 4]; 4],
}

fn main() {
    pollster::block_on(render_test());
}

async fn render_test() {
    // Setup wgpu without window
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .unwrap();

    // Create render target texture
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Render Target"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create vertex buffer and initialize with sentinel values
    let vertex_count = GRID_SIZE * GRID_SIZE;
    println!("Vertex size: {} bytes", std::mem::size_of::<Vertex>());
    println!("Vertex count: {}", vertex_count);
    println!(
        "Total buffer size: {} bytes",
        vertex_count as usize * std::mem::size_of::<Vertex>()
    );

    let init_vertices: Vec<Vertex> = (0..vertex_count)
        .map(|_i| Vertex {
            position: [-999.0, -999.0, -999.0],
            _padding1: 0.0,
            uv: [-1.0, -1.0],
            _padding2: [0.0, 0.0],
        })
        .collect();

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vertex Buffer"),
        size: (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&init_vertices));

    // Create terrain params
    let terrain_params = TerrainParams {
        base_amplitude: 100.0,
        base_frequency: 0.003,
        grid_size: GRID_SIZE,
        grid_spacing: 100.0,
    };

    let terrain_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Terrain Params"),
        size: std::mem::size_of::<TerrainParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(
        &terrain_params_buffer,
        0,
        bytemuck::bytes_of(&terrain_params),
    );

    // Load shaders
    let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("terrain_compute.wgsl").into()),
    });

    let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Render Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("terrain_render.wgsl").into()),
    });

    // Create compute pipeline
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

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
        cache: None,
    });

    // Create camera
    let extent = GRID_SIZE as f32 * terrain_params.grid_spacing;
    let scale = 2.0 / extent;
    let view_proj = [
        [scale, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, scale, 0.0, 0.0],
        [-1.0, -1.0, 0.0, 1.0],
    ];

    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Camera Buffer"),
        size: std::mem::size_of::<CameraUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(
        &camera_buffer,
        0,
        bytemuck::bytes_of(&CameraUniforms { view_proj }),
    );

    // Create render pipeline
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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
            compilation_options: Default::default(),
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
                        offset: 16, // After position + padding
                        shader_location: 1,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
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

    // Generate indices
    let indices = generate_indices(GRID_SIZE);
    println!("Grid size: {}, Total indices: {}", GRID_SIZE, indices.len());
    println!("First 40 indices: {:?}", &indices[..40.min(indices.len())]);

    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Index Buffer"),
        size: (indices.len() * std::mem::size_of::<u32>()) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

    // Render
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    // Compute pass
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &compute_bind_group, &[]);
        let workgroup_count = (vertex_count + 255) / 256;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    // Debug: Read back ALL vertices
    let debug_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Debug Buffer"),
        size: (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &vertex_buffer,
        0,
        &debug_buffer,
        0,
        (vertex_count as u64) * std::mem::size_of::<Vertex>() as u64,
    );

    queue.submit(Some(encoder.finish()));

    let debug_slice = debug_buffer.slice(..);
    debug_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);

    let debug_data = debug_slice.get_mapped_range();
    let vertices: &[Vertex] = bytemuck::cast_slice(&debug_data);
    println!("\nAll {} vertices after compute:", vertices.len());
    println!("First row (vertices 0-9):");
    for i in 0..10 {
        let v = &vertices[i];
        println!(
            "  V{}: pos=({}, {}, {})",
            i, v.position[0], v.position[1], v.position[2]
        );
    }
    println!("Second row (vertices 10-19):");
    for i in 10..20 {
        let v = &vertices[i];
        println!(
            "  V{}: pos=({}, {}, {})",
            i, v.position[0], v.position[1], v.position[2]
        );
    }

    // Find last written vertex
    let mut last_written = 0;
    for i in 0..vertices.len() {
        if vertices[i].position[0] != -999.0 {
            last_written = i;
        }
    }
    println!("\nLast written vertex: {}", last_written);

    println!("Vertices around index 74-75:");
    for i in 70..80 {
        let v = &vertices[i];
        println!(
            "  V{}: pos=({}, {}, {}), uv=({}, {})",
            i, v.position[0], v.position[1], v.position[2], v.uv[0], v.uv[1]
        );
    }
    drop(debug_data);
    debug_buffer.unmap();

    // Create new encoder for rendering
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    // Render pass
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
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

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    // Copy texture to buffer
    let bytes_per_row = WIDTH * 4;
    let unpadded_bytes_per_row = WIDTH * 4;
    let padded_bytes_per_row = ((unpadded_bytes_per_row + 255) / 256) * 256;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (padded_bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    // Read back and save
    let buffer_slice = output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);

    let data = buffer_slice.get_mapped_range();

    // Remove padding
    let mut image_data = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for row in 0..HEIGHT {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + bytes_per_row as usize;
        image_data.extend_from_slice(&data[start..end]);
    }

    image::save_buffer(
        "debug_wireframe.png",
        &image_data,
        WIDTH,
        HEIGHT,
        image::ColorType::Rgba8,
    )
    .unwrap();

    println!("Saved debug_wireframe.png");
}

fn generate_indices(grid_size: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    // Horizontal lines
    for z in 0..grid_size {
        for x in 0..grid_size - 1 {
            let i = z * grid_size + x;
            indices.push(i);
            indices.push(i + 1);
        }
    }
    // Vertical lines
    for z in 0..grid_size - 1 {
        for x in 0..grid_size {
            let i = z * grid_size + x;
            indices.push(i);
            indices.push(i + grid_size);
        }
    }
    indices
}
