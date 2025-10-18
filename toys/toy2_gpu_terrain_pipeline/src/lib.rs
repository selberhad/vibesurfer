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
    pub _padding2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view_proj: [[f32; 4]; 4],
}

// === Camera Math ===

pub fn create_perspective_view_proj_matrix(camera_z: f32, aspect: f32) -> [[f32; 4]; 4] {
    // Use glam for correct matrix math - proven implementation
    use glam::{Mat4, Vec3};

    // Camera position: 80m above terrain, centered horizontally, moving forward
    let eye = Vec3::new(512.0, 80.0, camera_z);

    // Look at: 300m ahead, 20m above terrain for horizon view
    let target = Vec3::new(512.0, 20.0, camera_z + 300.0);

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
