//! Noise generation for ocean terrain.
//!
//! Provides consistent noise implementation for both CPU (Rust) and GPU (WGSL).
//! Using OpenSimplex noise for smooth, artifact-free procedural terrain.

use noise::{NoiseFn, OpenSimplex};

/// Noise generator for ocean terrain
pub struct NoiseGenerator {
    simplex: OpenSimplex,
}

impl NoiseGenerator {
    /// Create new noise generator with seed
    pub fn new(seed: u32) -> Self {
        Self {
            simplex: OpenSimplex::new(seed),
        }
    }

    /// Sample 3D simplex noise at position
    ///
    /// Returns value in range [-1, 1]
    pub fn sample_3d(&self, x: f64, y: f64, z: f64) -> f32 {
        self.simplex.get([x, y, z]) as f32
    }
}
