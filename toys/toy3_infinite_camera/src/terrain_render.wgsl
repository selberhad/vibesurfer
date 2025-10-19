// Simple vertex/fragment shaders for terrain visualization

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
    torus_extent: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Toroidal wrapping: map torus-space vertex to nearest position relative to camera
    // Vertices are at torus positions (0 to torus_extent)
    // Camera is at unwrapped world position

    // Wrap camera to torus space to find which "tile" we're on
    let camera_torus_x = camera.camera_pos.x - floor(camera.camera_pos.x / camera.torus_extent) * camera.torus_extent;
    let camera_torus_z = camera.camera_pos.z - floor(camera.camera_pos.z / camera.torus_extent) * camera.torus_extent;

    // Calculate offset from camera (torus space)
    var dx = in.position.x - camera_torus_x;
    var dz = in.position.z - camera_torus_z;

    // Wrap to nearest distance
    let half_extent = camera.torus_extent * 0.5;
    if (dx > half_extent) { dx -= camera.torus_extent; }
    if (dx < -half_extent) { dx += camera.torus_extent; }
    if (dz > half_extent) { dz -= camera.torus_extent; }
    if (dz < -half_extent) { dz += camera.torus_extent; }

    // Unwrapped world position (what the camera actually sees)
    let world_pos = vec3<f32>(
        camera.camera_pos.x + dx,
        in.position.y,
        camera.camera_pos.z + dz
    );

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Cyan neon wireframe
    return vec4<f32>(0.0, 1.0, 1.0, 1.0);
}
