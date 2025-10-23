// Headless rendering test for spherical chunk streaming
use std::env;
use toy4_spherical_chunks::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn main() {
    // Parse command line arguments
    // Usage: test_render [start_angle] [end_angle] [step]
    // Example: test_render 0 3.14 0.5  -> renders at 0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0
    let args: Vec<String> = env::args().collect();

    let chunk_size = 256; // Fixed for now

    let (start_angle, end_angle, step) = if args.len() >= 4 {
        (
            args[1].parse::<f32>().unwrap_or(0.0),
            args[2].parse::<f32>().unwrap_or(3.0),
            args[3].parse::<f32>().unwrap_or(0.5),
        )
    } else if args.len() == 2 {
        // Single frame mode (backward compatibility)
        let angle = args[1].parse::<f32>().unwrap_or(0.0);
        (angle, angle, 1.0)
    } else {
        // Default: render frames from 0 to 3 radians in 0.5 rad steps
        (0.0, 3.0, 0.5)
    };

    // Generate sequence of angles
    let mut angles = Vec::new();
    let mut angle = start_angle;
    while angle <= end_angle + 0.001 {
        // Small epsilon for floating point comparison
        angles.push(angle);
        angle += step;
    }

    println!(
        "Rendering {} frames from {:.3}rad to {:.3}rad (step: {:.3}rad)",
        angles.len(),
        start_angle,
        end_angle,
        step
    );

    pollster::block_on(render_frames(angles, chunk_size));
}

async fn render_frames(angles: Vec<f32>, chunk_size: u32) {
    for angle in angles {
        render_frame(angle, chunk_size).await;
    }
}

async fn render_frame(camera_angle: f32, chunk_size: u32) {
    // Ensure screenshots directory exists
    std::fs::create_dir_all("screenshots").expect("Failed to create screenshots directory");

    // Setup wgpu headless
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

    // Create render target
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

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

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
        cache: Default::default(),
    });

    // Create chunk using lib
    let grid_spacing = 2.0;
    let chunk_extent_meters = chunk_size as f32 * grid_spacing;
    let chunk_angular_size = chunk_extent_meters / PLANET_RADIUS;

    let chunk_id = ChunkId::from_camera_angle(camera_angle, chunk_angular_size);
    let chunk = Chunk::create(
        &device,
        &queue,
        &compute_pipeline,
        &compute_bind_group_layout,
        chunk_id,
        chunk_size,
        grid_spacing,
        chunk_angular_size,
    );

    // Create camera using shared lib (ensures same altitude/orientation as main.rs)
    let camera = toy4_spherical_chunks::OrbitCamera::at_angle(
        toy4_spherical_chunks::DEFAULT_ALTITUDE,
        camera_angle,
    );
    let camera_uniforms = camera.camera_uniforms(WIDTH as f32 / HEIGHT as f32, false);

    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Camera Buffer"),
        size: std::mem::size_of::<CameraUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniforms));

    // Create render pipeline using shared lib
    let camera_bind_group_layout = toy4_spherical_chunks::create_camera_bind_group_layout(&device);

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let render_pipeline = toy4_spherical_chunks::create_render_pipeline(
        &device,
        &camera_bind_group_layout,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );

    // Render
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

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
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_bind_group(0, &camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
        render_pass.set_index_buffer(chunk.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..chunk.index_count, 0, 0..1);
    }

    queue.submit(std::iter::once(encoder.finish()));

    // Read back pixels
    let bytes_per_row = WIDTH * 4;
    let unpadded_bytes_per_row = WIDTH * 4;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (bytes_per_row * HEIGHT) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Copy Encoder"),
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
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Save to file
    let buffer_slice = output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);

    let data = buffer_slice.get_mapped_range();
    let mut png_data: Vec<u8> = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);

    for row in 0..HEIGHT {
        let start = (row * bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        png_data.extend_from_slice(&data[start..end]);
    }

    let filename = format!("screenshots/test_render_angle_{:.3}.png", camera_angle);
    image::save_buffer(&filename, &png_data, WIDTH, HEIGHT, image::ColorType::Rgba8).unwrap();

    println!("Saved: {}", filename);
}
