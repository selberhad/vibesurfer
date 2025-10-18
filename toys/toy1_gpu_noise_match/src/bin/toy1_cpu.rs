//! CPU reference noise generator
//!
//! Renders OpenSimplex noise to grayscale PNG heightmap for visual comparison.

use clap::Parser;
use image::{GrayImage, Luma};
use noise::{NoiseFn, OpenSimplex};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "toy1_cpu")]
#[command(about = "Generate CPU noise heightmap to PNG")]
struct Args {
    /// RNG seed for noise generation
    #[arg(long, default_value_t = 42)]
    seed: u32,

    /// Noise frequency (spatial scale)
    #[arg(long, default_value_t = 0.1)]
    frequency: f32,

    /// Output image size (width = height)
    #[arg(long, default_value_t = 256)]
    size: u32,

    /// Output file path
    #[arg(long, default_value = "output_cpu.png")]
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("CPU Noise Generator");
    println!("  Seed: {}", args.seed);
    println!("  Frequency: {}", args.frequency);
    println!("  Size: {}x{}", args.size, args.size);

    let start = Instant::now();

    // Initialize OpenSimplex noise with seed
    let simplex = OpenSimplex::new(args.seed);

    // Create grayscale image
    let mut img = GrayImage::new(args.size, args.size);

    // Sample noise in 2D grid (z=0 for 2D slice of 3D noise)
    for y in 0..args.size {
        for x in 0..args.size {
            // Scale coordinates by frequency
            let nx = x as f64 * args.frequency as f64;
            let ny = y as f64 * args.frequency as f64;

            // Sample 3D noise (z=0 for 2D heightmap)
            let noise_val = simplex.get([nx, ny, 0.0]);

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
