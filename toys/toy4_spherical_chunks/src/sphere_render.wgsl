// Vertex shader: Transform sphere vertices for rendering

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    _padding: f32,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.world_pos = in.position;
    return out;
}

// Fragment shader: Cyan neon wireframe with distance fog

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Distance from camera
    let distance = length(in.world_pos - camera.camera_pos);

    // Fog parameters (exponential for smooth falloff)
    let fog_density = 0.015;  // Controls fog thickness (higher = denser)
    let fog_color = vec3<f32>(0.0, 0.0, 0.0); // Black fog

    // Calculate exponential fog factor (0 = no fog, 1 = full fog)
    // exp2(-density * distance) gives smooth natural falloff
    let fog_factor = 1.0 - exp2(-fog_density * distance);

    // Base color (cyan neon)
    let base_color = vec3<f32>(0.0, 1.0, 1.0);

    // Mix base color with fog
    let final_color = mix(base_color, fog_color, fog_factor);

    return vec4<f32>(final_color, 1.0);
}
