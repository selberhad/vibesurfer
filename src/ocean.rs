//! Ocean surface simulation with procedural noise and audio-reactive modulation.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use noise::{NoiseFn, Perlin};

use crate::params::{AudioReactiveMapping, OceanPhysics};

/// Vertex data for ocean mesh (position + UV coordinates)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

/// Audio frequency band energies (shared between audio and rendering threads)
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioBands {
    pub low: f32,  // Bass (20-200 Hz)
    pub mid: f32,  // Mids (200-1000 Hz)
    pub high: f32, // Highs (1000-4000 Hz)
}

/// Ocean grid mesh with procedural noise animation
pub struct OceanGrid {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    perlin: Perlin,
}

impl OceanGrid {
    /// Create a new ocean grid with specified parameters
    pub fn new(physics: &OceanPhysics) -> Self {
        let grid_size = physics.grid_size;
        let grid_spacing = physics.grid_spacing_m;
        let half_size = (grid_size as f32 * grid_spacing) / 2.0;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate flat XZ plane grid
        for z in 0..=grid_size {
            for x in 0..=grid_size {
                let x_pos = x as f32 * grid_spacing - half_size;
                let z_pos = z as f32 * grid_spacing - half_size;

                vertices.push(Vertex {
                    position: [x_pos, 0.0, z_pos],
                    uv: [x as f32 / grid_size as f32, z as f32 / grid_size as f32],
                });
            }
        }

        // Generate triangle indices (counter-clockwise winding)
        for z in 0..grid_size {
            for x in 0..grid_size {
                let top_left = (z * (grid_size + 1) + x) as u32;
                let top_right = top_left + 1;
                let bottom_left = ((z + 1) * (grid_size + 1) + x) as u32;
                let bottom_right = bottom_left + 1;

                indices.extend_from_slice(&[
                    top_left,
                    bottom_left,
                    top_right,
                    top_right,
                    bottom_left,
                    bottom_right,
                ]);
            }
        }

        Self {
            vertices,
            indices,
            perlin: Perlin::new(physics.noise_seed),
        }
    }

    /// Update ocean surface with Perlin noise animation
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `amplitude_m` - Wave height in meters
    /// * `frequency` - Spatial frequency (cycles per meter)
    /// * `camera_pos` - Camera position (for infinite ocean illusion)
    /// * `physics` - Ocean physics parameters
    pub fn update(
        &mut self,
        time_s: f32,
        amplitude_m: f32,
        frequency: f32,
        camera_pos: Vec3,
        physics: &OceanPhysics,
    ) {
        let t = time_s * physics.wave_speed;

        for vertex in &mut self.vertices {
            // Local grid coordinates
            let x_local = vertex.position[0];
            let z_local = vertex.position[2];

            // World coordinates - offset by camera XZ to create infinite ocean illusion
            let x_world = x_local + camera_pos.x;
            let z_world = z_local + camera_pos.z;

            // Sample 3D Perlin noise (x, z, time)
            let noise_value = self.perlin.get([
                (x_world * frequency) as f64,
                (z_world * frequency) as f64,
                t as f64,
            ]) as f32;

            // Set Y position (wave height)
            vertex.position[1] = noise_value * amplitude_m;
        }
    }
}

/// High-level ocean system with physics and audio-reactive parameters
pub struct OceanSystem {
    pub grid: OceanGrid,
    physics: OceanPhysics,
    mapping: AudioReactiveMapping,
}

impl OceanSystem {
    /// Create new ocean system with specified parameters
    pub fn new(physics: OceanPhysics, mapping: AudioReactiveMapping) -> Self {
        let grid = OceanGrid::new(&physics);
        Self {
            grid,
            physics,
            mapping,
        }
    }

    /// Update ocean simulation with audio-reactive modulation
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `audio_bands` - FFT frequency band energies
    /// * `camera_pos` - Camera position for infinite ocean
    ///
    /// # Returns
    /// * Tuple of (amplitude, frequency, line_width) for rendering
    pub fn update(
        &mut self,
        time_s: f32,
        audio_bands: &AudioBands,
        camera_pos: Vec3,
    ) -> (f32, f32, f32) {
        // Map audio bands to ocean parameters
        let amplitude =
            self.physics.base_amplitude_m + audio_bands.low * self.mapping.bass_to_amplitude_scale;

        let frequency =
            self.physics.base_frequency + audio_bands.mid * self.mapping.mid_to_frequency_scale;

        let line_width =
            self.physics.base_line_width + audio_bands.high * self.mapping.high_to_glow_scale;

        // Update mesh vertices
        self.grid
            .update(time_s, amplitude, frequency, camera_pos, &self.physics);

        (amplitude, frequency, line_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocean_grid_creation() {
        let physics = OceanPhysics::default();
        let grid = OceanGrid::new(&physics);

        // Check vertex count: (grid_size + 1)^2
        assert_eq!(grid.vertices.len(), (physics.grid_size + 1).pow(2));

        // Check triangle count: grid_size^2 * 2 triangles * 3 indices
        assert_eq!(grid.indices.len(), physics.grid_size.pow(2) * 6);
    }

    #[test]
    fn test_audio_reactive_mapping() {
        let physics = OceanPhysics::default();
        let mapping = AudioReactiveMapping::default();
        let mut ocean = OceanSystem::new(physics, mapping);

        let bands = AudioBands {
            low: 1.0,
            mid: 0.5,
            high: 0.2,
        };

        let (amplitude, frequency, line_width) = ocean.update(0.0, &bands, Vec3::ZERO);

        // Check that audio modulation is applied
        assert!(amplitude > ocean.physics.base_amplitude_m);
        assert!(frequency > ocean.physics.base_frequency);
        assert!(line_width > ocean.physics.base_line_width);
    }
}
