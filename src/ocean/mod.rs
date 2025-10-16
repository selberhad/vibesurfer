//! Ocean surface simulation with procedural noise and audio-reactive modulation.

mod mesh;
mod system;

// Re-export public types
pub use mesh::{OceanGrid, Vertex};
pub use system::OceanSystem;

/// Audio frequency band energies (shared between audio and rendering threads)
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioBands {
    pub low: f32,  // Bass (20-200 Hz)
    pub mid: f32,  // Mids (200-1000 Hz)
    pub high: f32, // Highs (1000-4000 Hz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::OceanPhysics;

    #[test]
    fn test_ocean_grid_creation() {
        let physics = OceanPhysics::default();
        let grid = OceanGrid::new(&physics);

        // Check vertex count: (grid_size + 1)^2
        assert_eq!(grid.vertices.len(), (physics.grid_size + 1).pow(2));

        // Check triangle count: grid_size^2 * 2 triangles * 3 indices
        assert_eq!(grid.indices.len(), physics.grid_size.pow(2) * 6);
    }
}
