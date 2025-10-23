// Vertex shader: Transform sphere vertices for rendering

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    debug_chunk_boundaries: u32,  // 0 = off, 1 = on
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) grid_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) grid_coord: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.world_pos = in.position;
    out.normal = in.normal;
    out.grid_coord = in.grid_coord;
    return out;
}

// Fragment shader: Lit surface with wireframe overlay

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.world_pos - camera.camera_pos);

    // Cyan wireframe color
    let wireframe_color = vec3<f32>(0.0, 1.0, 1.0);

    // Exponential fog
    let fog_factor = 1.0 - exp2(-0.015 * distance);
    let final_color = mix(wireframe_color, vec3<f32>(0.0, 0.0, 0.0), fog_factor);

    return vec4<f32>(final_color, 1.0);
}
