// Ocean vertex update compute shader
// Computes detail layer Perlin noise on GPU

struct Vertex {
    position: vec3<f32>,
    uv: vec2<f32>,
}

struct ComputeParams {
    camera_pos: vec3<f32>,
    detail_amplitude: f32,
    detail_frequency: f32,
    time: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read> base_heights: array<f32>;
@group(0) @binding(2) var<uniform> params: ComputeParams;

// Simple Perlin-like noise (simplified for GPU)
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p3_dot = dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3_dot);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // Smoothstep

    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Bounds check
    if (idx >= arrayLength(&vertices)) {
        return;
    }

    let vertex = vertices[idx];

    // World position
    let x_world = params.camera_pos.x + vertex.position.x;
    let z_world = params.camera_pos.z + vertex.position.z;

    // Detail layer noise (simplified Perlin)
    let noise_pos = vec2<f32>(x_world, z_world) * params.detail_frequency;
    let detail_noise = noise(noise_pos + vec2<f32>(params.time));
    let detail_height = detail_noise * params.detail_amplitude;

    // Combine base (from CPU cache) + detail
    let base_height = base_heights[idx];
    vertices[idx].position.y = base_height + detail_height;
}
