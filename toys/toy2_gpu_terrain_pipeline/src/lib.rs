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
    // Perspective camera: hover above terrain, looking forward
    // Camera positioned 100m above, 200m behind current position
    let camera_pos = [0.0, 100.0, camera_z - 200.0];
    let look_at = [0.0, 0.0, camera_z + 300.0]; // Look 300m ahead
    let up = [0.0, 1.0, 0.0];

    // View matrix (look-at)
    let forward = [
        look_at[0] - camera_pos[0],
        look_at[1] - camera_pos[1],
        look_at[2] - camera_pos[2],
    ];
    let forward_len =
        (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
    let forward = [
        forward[0] / forward_len,
        forward[1] / forward_len,
        forward[2] / forward_len,
    ];

    let right = [
        forward[1] * up[2] - forward[2] * up[1],
        forward[2] * up[0] - forward[0] * up[2],
        forward[0] * up[1] - forward[1] * up[0],
    ];
    let right_len = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
    let right = [
        right[0] / right_len,
        right[1] / right_len,
        right[2] / right_len,
    ];

    let camera_up = [
        right[1] * forward[2] - right[2] * forward[1],
        right[2] * forward[0] - right[0] * forward[2],
        right[0] * forward[1] - right[1] * forward[0],
    ];

    let view = [
        [right[0], camera_up[0], -forward[0], 0.0],
        [right[1], camera_up[1], -forward[1], 0.0],
        [right[2], camera_up[2], -forward[2], 0.0],
        [
            -(right[0] * camera_pos[0] + right[1] * camera_pos[1] + right[2] * camera_pos[2]),
            -(camera_up[0] * camera_pos[0]
                + camera_up[1] * camera_pos[1]
                + camera_up[2] * camera_pos[2]),
            -(-forward[0] * camera_pos[0]
                + -forward[1] * camera_pos[1]
                + -forward[2] * camera_pos[2]),
            1.0,
        ],
    ];

    // Perspective projection (60° FOV)
    let fov_y = 60.0_f32.to_radians();
    let f = 1.0 / (fov_y / 2.0).tan();
    let near = 1.0;
    let far = 10000.0;

    let proj = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, (near * far) / (near - far), 0.0],
    ];

    // Combine view * proj
    multiply_matrix_4x4(&proj, &view)
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
