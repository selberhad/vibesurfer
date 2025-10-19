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

    // Toroidal wrapping: position vertices relative to camera
    // This creates seamless infinite terrain by replicating the torus
    let torus_camera_x = camera.camera_pos.x - floor(camera.camera_pos.x / camera.torus_extent) * camera.torus_extent;
    let torus_camera_z = camera.camera_pos.z - floor(camera.camera_pos.z / camera.torus_extent) * camera.torus_extent;

    // Calculate offset from camera (with wrapping for shortest distance)
    var dx = in.position.x - torus_camera_x;
    var dz = in.position.z - torus_camera_z;

    // Wrap to nearest distance
    let half_extent = camera.torus_extent * 0.5;
    if (dx > half_extent) { dx -= camera.torus_extent; }
    if (dx < -half_extent) { dx += camera.torus_extent; }
    if (dz > half_extent) { dz -= camera.torus_extent; }
    if (dz < -half_extent) { dz += camera.torus_extent; }

    // Position relative to camera (unwrapped world space)
    let world_pos = vec3<f32>(torus_camera_x + dx, in.position.y, torus_camera_z + dz);

    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Cyan neon wireframe
    return vec4<f32>(0.0, 1.0, 1.0, 1.0);
}
