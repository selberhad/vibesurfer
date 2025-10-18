//! GPU compute noise generator
//!
//! Renders simplex noise using GPU compute shader to grayscale PNG heightmap.

use bytemuck::{Pod, Zeroable};
use clap::Parser;
use image::{GrayImage, Luma};
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct NoiseParams {
    frequency: f32,
    size: u32,
    _padding: [f32; 2],
}

#[derive(Parser, Debug)]
#[command(name = "toy1_gpu")]
#[command(about = "Generate GPU noise heightmap to PNG")]
struct Args {
    /// RNG seed for noise generation (Note: GPU shader doesn't support seed yet)
    #[arg(long, default_value_t = 42)]
    seed: u32,

    /// Noise frequency (spatial scale)
    #[arg(long, default_value_t = 0.1)]
    frequency: f32,

    /// Output image size (width = height)
    #[arg(long, default_value_t = 256)]
    size: u32,

    /// Output file path
    #[arg(long, default_value = "output_gpu.png")]
    output: String,
}

async fn run_gpu_noise(args: &Args) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Initialize wgpu
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
        .ok_or("Failed to find GPU adapter")?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("GPU Noise Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await?;

    // Load compute shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Noise Compute Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../noise.wgsl").into()),
    });

    // Create output buffer (size x size floats)
    let output_size = (args.size * args.size * std::mem::size_of::<f32>() as u32) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create staging buffer for readback
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create uniform buffer for params
    let params = NoiseParams {
        frequency: args.frequency,
        size: args.size,
        _padding: [0.0; 2],
    };
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Compute Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: output_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    // Create compute pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Noise Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    // Dispatch compute shader
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Noise Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Dispatch: (size/16, size/16) workgroups (16×16 threads per workgroup)
        let workgroup_count_x = (args.size + 15) / 16;
        let workgroup_count_y = (args.size + 15) / 16;
        compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
    }

    // Copy output to staging buffer
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);

    queue.submit(Some(encoder.finish()));

    // Read back results
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures::channel::oneshot::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    device.poll(wgpu::Maintain::Wait);
    receiver.await??;

    let data = buffer_slice.get_mapped_range();
    let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

    drop(data);
    staging_buffer.unmap();

    Ok(result)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("GPU Noise Generator");
    println!(
        "  Seed: {} (Note: seed not yet implemented in shader)",
        args.seed
    );
    println!("  Frequency: {}", args.frequency);
    println!("  Size: {}x{}", args.size, args.size);

    let start = Instant::now();

    // Run GPU compute
    let noise_values = pollster::block_on(run_gpu_noise(&args))?;

    // Create grayscale image from GPU output
    let mut img = GrayImage::new(args.size, args.size);

    for y in 0..args.size {
        for x in 0..args.size {
            let idx = (y * args.size + x) as usize;
            let noise_val = noise_values[idx];

            // Map noise from [-1, 1] to grayscale [0, 255]
            let gray = ((noise_val + 1.0) * 127.5).clamp(0.0, 255.0) as u8;

            img.put_pixel(x, y, Luma([gray]));
        }
    }

    // Save PNG
    img.save(&args.output)?;

    let elapsed = start.elapsed();
    println!("  Output: {}", args.output);
    println!("  Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

    Ok(())
}
