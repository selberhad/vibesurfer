// GPU Terrain Generation Compute Shader
// Generates procedural heightfield using 3D simplex noise

struct Vertex {
    position: vec3<f32>,
    _padding1: f32,  // Align position to 16 bytes
    uv: vec2<f32>,
    _padding2: vec2<f32>,  // Pad struct to 32 bytes total for array alignment
}

struct TerrainParams {
    base_amplitude: f32,      // meters (e.g., 150.0)
    base_frequency: f32,      // cycles/meter (e.g., 0.003)
    detail_amplitude: f32,    // audio-modulated detail height
    detail_frequency: f32,    // audio-modulated choppiness
    camera_pos: vec3<f32>,    // world-space camera position
    _padding1: f32,           // Align camera_pos to 16 bytes
    grid_size: u32,           // vertices per side (512 or 1024)
    grid_spacing: f32,        // meters between vertices (2.0)
    time: f32,                // seconds (for animation)
    _padding2: f32,
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<uniform> params: TerrainParams;

// === 3D Simplex Noise (Stefan Gustavson) ===

fn mod289_vec3(x: vec3<f32>) -> vec3<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn mod289_vec4(x: vec4<f32>) -> vec4<f32> {
    return x - floor(x * (1.0 / 289.0)) * 289.0;
}

fn permute(x: vec4<f32>) -> vec4<f32> {
    return mod289_vec4(((x * 34.0) + 1.0) * x);
}

fn taylorInvSqrt(r: vec4<f32>) -> vec4<f32> {
    return 1.79284291400159 - 0.85373472095314 * r;
}

fn simplex3d(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0/6.0, 1.0/3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);

    // First corner
    var i = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);

    // Other corners
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);

    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;

    // Permutations
    i = mod289_vec3(i);
    let p = permute(permute(permute(
        i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
        + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0))
        + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));

    // Gradients
    let n_ = 0.142857142857; // 1.0/7.0
    let ns = n_ * D.wyz - D.xzx;

    let j = p - 49.0 * floor(p * ns.z * ns.z);

    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);

    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);

    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);

    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));

    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;

    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);

    // Normalize gradients
    let norm = taylorInvSqrt(vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3)));
    p0 *= norm.x;
    p1 *= norm.y;
    p2 *= norm.z;
    p3 *= norm.w;

    // Mix final noise value
    var m = max(0.6 - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), vec4<f32>(0.0));
    m = m * m;
    return 42.0 * dot(m * m, vec4<f32>(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}

// === Main Compute Kernel ===

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Bounds check
    let grid_size = params.grid_size;
    let total_vertices = grid_size * grid_size;
    if (idx >= total_vertices) {
        return;
    }

    // Calculate grid position from linear index
    let x = idx % grid_size;
    let z = idx / grid_size;

    // Calculate world-space position
    let world_x = f32(x) * params.grid_spacing;
    let world_z = f32(z) * params.grid_spacing;

    // Sample base terrain (static, large hills)
    let base_coord_x = world_x * 0.1 * params.base_frequency;
    let base_coord_z = world_z * 0.1 * params.base_frequency;
    let base_height = simplex3d(vec3<f32>(base_coord_x, base_coord_z, 0.0)) * params.base_amplitude;

    // Sample detail layer (animated, audio-reactive)
    let detail_coord_x = world_x * 0.1 * params.detail_frequency;
    let detail_coord_z = world_z * 0.1 * params.detail_frequency;
    let detail_height = simplex3d(vec3<f32>(detail_coord_x, detail_coord_z, params.time)) * params.detail_amplitude;

    // Combine layers
    let height = base_height + detail_height;

    // Write vertex data
    vertices[idx].position = vec3<f32>(world_x, height, world_z);
    vertices[idx].uv = vec2<f32>(f32(x) / f32(grid_size), f32(z) / f32(grid_size));
}
