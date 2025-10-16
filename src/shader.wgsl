struct Uniforms {
    view_proj: mat4x4<f32>,
    line_width: f32,
    amplitude: f32,
    frequency: f32,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let hot_pink = vec3<f32>(1.0, 0.16, 0.46);
    let deep_purple = vec3<f32>(0.55, 0.12, 1.0);
    let electric_blue = vec3<f32>(0.0, 0.8, 1.0);

    // Create grid pattern
    let uv_scaled = in.uv * 16.0;
    let grid = fract(uv_scaled);
    let dist_x = min(grid.x, 1.0 - grid.x);
    let dist_y = min(grid.y, 1.0 - grid.y);
    let dist = min(dist_x, dist_y);

    // Gradient from hot pink to deep purple
    let gradient_t = in.uv.y;
    var color = mix(hot_pink, deep_purple, gradient_t);

    // Add electric blue highlights on horizontal lines
    if dist_y < dist_x {
        color = mix(color, electric_blue, 0.3);
    }

    // Smooth glow effect using AUDIO-REACTIVE line_width!
    let core_intensity = 1.0 - smoothstep(0.0, uniforms.line_width * 0.3, dist);
    let glow_intensity = 1.0 - smoothstep(0.0, uniforms.line_width * 3.0, dist);
    let brightness = core_intensity * 2.5 + glow_intensity * 0.8;

    color = color * brightness;

    // Distance-based fade to create circular ocean view AND hide wrap boundary
    let dist_from_center = length(in.world_pos.xz);
    let fade_start = 800.0;  // Start fading farther out (1024×1024 grid)
    let fade_end = 1000.0;   // Complete fade before wrap boundary (1024m)
    let distance_fade = 1.0 - smoothstep(fade_start, fade_end, dist_from_center);

    // Output with translucency and distance fade
    let alpha = clamp(brightness, 0.0, 1.0) * distance_fade;
    return vec4<f32>(color, alpha);
}
