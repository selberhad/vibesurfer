// Shared terrain rendering library

// === Helper Functions ===

pub fn multiply_matrix_4x4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

// === Data Structures ===

use std::time::Instant;

/// Camera state for toroidal navigation
pub struct CameraState {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    last_update: Instant,
}

impl CameraState {
    pub fn new(position: [f32; 3], velocity: [f32; 3]) -> Self {
        Self {
            position,
            velocity,
            last_update: Instant::now(),
        }
    }

    /// Update camera position based on velocity and delta time
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_update).as_secs_f32();

        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
        self.position[2] += self.velocity[2] * dt;

        self.last_update = now;
    }

    /// Set velocity (for keyboard input, etc.)
    pub fn set_velocity(&mut self, velocity: [f32; 3]) {
        self.velocity = velocity;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub _padding1: f32, // Align position to 16 bytes
    pub uv: [f32; 2],
    pub _padding2: [f32; 2], // Pad to 32 bytes for WGSL storage array alignment
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainParams {
    pub base_amplitude: f32,
    pub base_frequency: f32,
    pub detail_amplitude: f32,
    pub detail_frequency: f32,
    pub camera_pos: [f32; 3],
    pub _padding1: f32, // Align camera_pos to 16 bytes
    pub grid_size: u32,
    pub grid_spacing: f32,
    pub time: f32,
    pub torus_extent_x: f32,
    pub torus_extent_z: f32,
    pub _padding2: f32,
    pub _padding3: f32,
    pub _padding4: f32,
    pub _padding5: f32,
    pub _padding6: f32,
    pub _padding7: f32,
    pub _padding8: f32, // Pad to 80 bytes total
}

impl TerrainParams {
    /// Create terrain params with default values
    pub fn new(grid_size: u32, grid_spacing: f32, camera_pos: [f32; 3], time: f32) -> Self {
        Self {
            base_amplitude: 100.0,
            base_frequency: 0.003,
            detail_amplitude: 2.0,
            detail_frequency: 0.1,
            camera_pos,
            _padding1: 0.0,
            grid_size,
            grid_spacing,
            time,
            torus_extent_x: grid_spacing * grid_size as f32,
            torus_extent_z: grid_spacing * grid_size as f32,
            _padding2: 0.0,
            _padding3: 0.0,
            _padding4: 0.0,
            _padding5: 0.0,
            _padding6: 0.0,
            _padding7: 0.0,
            _padding8: 0.0,
        }
    }

    /// Update audio-reactive parameters
    pub fn with_audio(&mut self, low: f32, mid: f32) -> &mut Self {
        self.detail_amplitude = 2.0 + low * 3.0;
        self.detail_frequency = 0.1 + mid * 0.15;
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
}

// === Camera Math ===

pub fn create_perspective_view_proj_matrix(aspect: f32) -> [[f32; 4]; 4] {
    // Use glam for correct matrix math - proven implementation
    use glam::{Mat4, Vec3};

    // Camera at origin in view space (vertices are already camera-relative)
    let eye = Vec3::new(0.0, 80.0, 0.0);

    // Look ahead and down for horizon view
    let target = Vec3::new(0.0, 20.0, 300.0);

    // World up
    let up = Vec3::Y;

    // Build view matrix using glam's look_at_rh (right-handed)
    let view = Mat4::look_at_rh(eye, target, up);

    // Perspective projection: 60° FOV, aspect ratio, near=1m, far=2000m
    let proj = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 1.0, 2000.0);

    // Combine and return as array
    (proj * view).to_cols_array_2d()
}

// === Index Generation ===

pub fn generate_grid_indices(grid_size: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    // Generate line segments for a wireframe grid
    // Horizontal lines (connect vertices in same row)
    for z in 0..grid_size {
        for x in 0..grid_size - 1 {
            let i = z * grid_size + x;
            indices.push(i);
            indices.push(i + 1);
        }
    }
    // Vertical lines (connect vertices in same column)
    for z in 0..grid_size - 1 {
        for x in 0..grid_size {
            let i = z * grid_size + x;
            indices.push(i);
            indices.push(i + grid_size);
        }
    }
    indices
}
