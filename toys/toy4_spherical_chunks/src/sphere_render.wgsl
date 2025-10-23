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
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.world_pos = in.position;
    out.normal = in.normal;
    return out;
}

// Fragment shader: Lit surface with wireframe overlay

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let distance = length(in.world_pos - camera.camera_pos);

    // Directional light
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let diffuse = max(dot(normal, light_dir), 0.0);
    let lighting = 0.2 + 0.8 * diffuse;

    // Dark teal surface
    let surface_color = vec3<f32>(0.0, 0.15, 0.2) * lighting;

    // Wireframe overlay (UV-based grid)
    let grid_freq = 256.0;
    let uv_grid = fract(in.uv * grid_freq);
    let thickness = 0.02;
    let edge = step(uv_grid.x, thickness) + step(1.0 - thickness, uv_grid.x) +
               step(uv_grid.y, thickness) + step(1.0 - thickness, uv_grid.y);
    let wireframe = clamp(edge, 0.0, 1.0);
    let wireframe_color = vec3<f32>(0.0, 1.0, 1.0) * (0.5 + 0.5 * lighting);

    var base_color = mix(surface_color, wireframe_color, wireframe);

    // Debug: Chunk boundary visualization (red borders)
    if (camera.debug_chunk_boundaries != 0u) {
        let chunk_edge_x = step(in.uv.x, 0.01) + step(0.99, in.uv.x);
        let chunk_edge_y = step(in.uv.y, 0.01) + step(0.99, in.uv.y);
        let chunk_boundary = clamp(chunk_edge_x + chunk_edge_y, 0.0, 1.0);
        let boundary_color = vec3<f32>(1.0, 0.0, 0.0);
        base_color = mix(base_color, boundary_color, chunk_boundary);
    }

    // Exponential fog
    let fog_factor = 1.0 - exp2(-0.015 * distance);
    let final_color = mix(base_color, vec3<f32>(0.0, 0.0, 0.0), fog_factor);

    return vec4<f32>(final_color, 1.0);
}
