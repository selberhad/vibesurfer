//! Ocean grid mesh with procedural noise animation and toroidal wrapping.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use noise::{NoiseFn, Perlin};

use crate::params::OceanPhysics;

/// Vertex data for ocean mesh (position + UV coordinates)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

/// Ocean grid mesh with procedural noise animation
pub struct OceanGrid {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    /// Filtered indices (excludes stretched triangles from wrapping)
    pub filtered_indices: Vec<u32>,
    perlin: Perlin,
    grid_size: usize,
    grid_spacing: f32,
    /// Last camera position (for computing delta movement)
    last_camera_pos: Vec3,
    /// Base terrain heights (stable physics surface, not affected by audio)
    base_terrain_heights: Vec<f32>,
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

        let vertex_count = vertices.len();
        let filtered_indices = indices.clone(); // Initially same as indices

        Self {
            vertices,
            indices,
            filtered_indices,
            perlin: Perlin::new(physics.noise_seed),
            grid_size: physics.grid_size,
            grid_spacing: physics.grid_spacing_m,
            last_camera_pos: Vec3::ZERO,
            base_terrain_heights: vec![0.0; vertex_count],
        }
    }

    /// Query base terrain height at world position (for physics)
    ///
    /// Returns stable terrain height without audio-reactive detail.
    /// Used for player collision, skiing physics, etc.
    #[allow(dead_code)] // Reserved for future physics system
    pub fn query_base_terrain(&self, world_x: f32, world_z: f32, physics: &OceanPhysics) -> f32 {
        let t = 0.0; // Base terrain is time-independent (static hills)

        let noise_value = self.perlin.get([
            (world_x * physics.base_terrain_frequency) as f64,
            (world_z * physics.base_terrain_frequency) as f64,
            t as f64,
        ]) as f32;

        noise_value * physics.base_terrain_amplitude_m
    }

    /// Update ocean surface with two-layer terrain system
    ///
    /// Layer 1 (Base terrain): Stable large-scale hills for skiing physics
    /// Layer 2 (Detail): Audio-reactive ripples for visual interest
    ///
    /// Uses flowing surface approach: grid vertices scroll backward as camera "moves" forward,
    /// with toroidal wrapping to create infinite extent illusion.
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `detail_amplitude_m` - Detail wave height (audio-modulated)
    /// * `detail_frequency` - Detail spatial frequency
    /// * `camera_pos` - Camera position (used to compute flow velocity)
    /// * `physics` - Ocean physics parameters
    pub fn update(
        &mut self,
        time_s: f32,
        detail_amplitude_m: f32,
        detail_frequency: f32,
        camera_pos: Vec3,
        physics: &OceanPhysics,
    ) {
        let detail_t = time_s * physics.wave_speed;

        // Compute camera delta (how much camera moved this frame)
        let camera_delta = camera_pos - self.last_camera_pos;
        self.last_camera_pos = camera_pos;

        // Grid dimensions for wrapping
        let grid_world_size = self.grid_size as f32 * self.grid_spacing;
        let half_size = grid_world_size / 2.0;

        // Flow grid backward opposite to camera motion
        // (Camera moves forward → grid flows backward)
        for (idx, vertex) in self.vertices.iter_mut().enumerate() {
            // Move vertex opposite to camera motion
            vertex.position[0] -= camera_delta.x;
            vertex.position[2] -= camera_delta.z;

            // Toroidal wrapping: if vertex exits behind camera, wrap to front
            // Wrap in Z (forward/backward)
            if vertex.position[2] < -half_size {
                vertex.position[2] += grid_world_size;
            } else if vertex.position[2] > half_size {
                vertex.position[2] -= grid_world_size;
            }

            // Wrap in X (left/right)
            if vertex.position[0] < -half_size {
                vertex.position[0] += grid_world_size;
            } else if vertex.position[0] > half_size {
                vertex.position[0] -= grid_world_size;
            }

            // Get absolute world coordinates
            let x_world = camera_pos.x + vertex.position[0];
            let z_world = camera_pos.z + vertex.position[2];

            // Layer 1: Base terrain (stable, time-independent hills)
            let base_noise = self.perlin.get([
                (x_world * physics.base_terrain_frequency) as f64,
                (z_world * physics.base_terrain_frequency) as f64,
                0.0, // Time-independent for stable terrain
            ]) as f32;
            let base_height = base_noise * physics.base_terrain_amplitude_m;
            self.base_terrain_heights[idx] = base_height;

            // Layer 2: Detail (audio-reactive, animated)
            let detail_noise = self.perlin.get([
                (x_world * detail_frequency) as f64,
                (z_world * detail_frequency) as f64,
                detail_t as f64,
            ]) as f32;
            let detail_height = detail_noise * detail_amplitude_m;

            // Combine layers for visual rendering
            vertex.position[1] = base_height + detail_height;
        }

        // Filter out stretched triangles (from toroidal wrapping)
        self.filter_stretched_triangles();
    }

    /// Filter indices to remove stretched triangles caused by vertex wrapping
    ///
    /// Triangles with any edge longer than threshold are excluded from rendering.
    /// This prevents "phantom lines" from wrapped vertices.
    fn filter_stretched_triangles(&mut self) {
        // Threshold: any edge longer than this is considered stretched
        // Use 10x grid spacing as reasonable max edge length
        let max_edge_length = self.grid_spacing * 10.0;
        let max_edge_sq = max_edge_length * max_edge_length; // Use squared distance (cheaper)

        self.filtered_indices.clear();

        // Check each triangle
        for tri in self.indices.chunks(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            let v0 = Vec3::from_array(self.vertices[i0].position);
            let v1 = Vec3::from_array(self.vertices[i1].position);
            let v2 = Vec3::from_array(self.vertices[i2].position);

            // Check all three edges
            let edge1_sq = v0.distance_squared(v1);
            let edge2_sq = v1.distance_squared(v2);
            let edge3_sq = v2.distance_squared(v0);

            // Keep triangle only if all edges are reasonable length
            if edge1_sq < max_edge_sq && edge2_sq < max_edge_sq && edge3_sq < max_edge_sq {
                self.filtered_indices.extend_from_slice(tri);
            }
        }
    }
}
