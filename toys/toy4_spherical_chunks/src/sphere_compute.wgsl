// Compute shader: Project flat grid to sphere surface with noise-based terrain

struct Vertex {
    position: vec3<f32>,
    _padding1: f32,
    uv: vec2<f32>,
    _padding2: vec2<f32>,
    normal: vec3<f32>,
    _padding3: f32,
}

struct SphereParams {
    planet_radius: f32,
    chunk_origin_lon_cell: i32,  // Global grid cell X coordinate
    chunk_origin_lat_cell: i32,  // Global grid cell Z coordinate
    grid_size: u32,
    grid_spacing: f32,
    base_amplitude: f32,    // Height variation (meters)
    base_frequency: f32,    // Noise scale
    detail_amplitude: f32,  // Detail layer height
    detail_frequency: f32,  // Detail layer scale
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<uniform> params: SphereParams;

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

// Helper: Calculate position using global integer grid coordinates
// Key: All chunks calculate positions from the same global grid origin
// This guarantees bitwise-identical positions for edge vertices
fn get_position(gx: u32, gz: u32) -> vec3<f32> {
    // Calculate GLOBAL grid coordinates (same for all chunks at this position)
    let global_x = params.chunk_origin_lon_cell + i32(gx);
    let global_z = params.chunk_origin_lat_cell + i32(gz);

    // Convert to world space meters (identical math for all chunks)
    let world_x = f32(global_x) * params.grid_spacing;
    let world_z = f32(global_z) * params.grid_spacing;

    // Convert to spherical coordinates
    let lon = world_x / params.planet_radius;
    let lat = world_z / params.planet_radius;

    // Sample noise at global coordinates
    let base_noise = simplex3d(vec3<f32>(lon * params.base_frequency, lat * params.base_frequency, 0.0));
    let detail_noise = simplex3d(vec3<f32>(lon * params.detail_frequency, lat * params.detail_frequency, 100.0));
    let height = base_noise * params.base_amplitude + detail_noise * params.detail_amplitude;

    // Project to sphere surface
    let r = params.planet_radius + height;
    return vec3<f32>(r * cos(lat) * cos(lon), r * sin(lat), r * cos(lat) * sin(lon));
}

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.grid_size * params.grid_size) {
        return;
    }

    let grid_x = idx % params.grid_size;
    let grid_z = idx / params.grid_size;
    let position = get_position(grid_x, grid_z);

    // Calculate normal via finite differences
    var dx: vec3<f32>;
    var dz: vec3<f32>;

    if (grid_x == 0u) {
        dx = get_position(1u, grid_z) - position;
    } else if (grid_x == params.grid_size - 1u) {
        dx = position - get_position(grid_x - 1u, grid_z);
    } else {
        dx = get_position(grid_x + 1u, grid_z) - get_position(grid_x - 1u, grid_z);
    }

    if (grid_z == 0u) {
        dz = get_position(grid_x, 1u) - position;
    } else if (grid_z == params.grid_size - 1u) {
        dz = position - get_position(grid_x, grid_z - 1u);
    } else {
        dz = get_position(grid_x, grid_z + 1u) - get_position(grid_x, grid_z - 1u);
    }

    // Analytical sphere normal (direction from planet center)
    // More robust than finite differences across chunk boundaries
    let normal = normalize(position);

    vertices[idx].position = position;
    vertices[idx].uv = vec2<f32>(f32(grid_x) / f32(params.grid_size - 1), f32(grid_z) / f32(params.grid_size - 1));
    vertices[idx].normal = normal;
}
