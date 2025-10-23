// Compute shader: Project flat grid to sphere surface with noise-based terrain

struct Vertex {
    position: vec3<f32>,
    _padding1: f32,
    uv: vec2<f32>,
    _padding2: vec2<f32>,
}

struct SphereParams {
    planet_radius: f32,
    chunk_center_lat: f32,
    chunk_center_lon: f32,
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

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_vertices = params.grid_size * params.grid_size;

    if (idx >= total_vertices) {
        return;
    }

    // Convert 1D index to 2D grid coordinates
    let grid_x = idx % params.grid_size;
    let grid_z = idx / params.grid_size;

    // Local chunk coordinates (flat grid centered at origin)
    let half_size = f32(params.grid_size - 1) * params.grid_spacing * 0.5;
    let local_x = f32(grid_x) * params.grid_spacing - half_size;
    let local_z = f32(grid_z) * params.grid_spacing - half_size;

    // Convert to angular offsets from chunk center
    // Arc length = radius * angle, so angle = arc_length / radius
    let lat_offset = local_z / params.planet_radius;
    let lon_offset = local_x / params.planet_radius;

    let lat = params.chunk_center_lat + lat_offset;
    let lon = params.chunk_center_lon + lon_offset;

    // Sample noise at spherical coordinates (globally consistent terrain)
    // Use lat/lon as 2D coordinates, with a third dimension for variation
    let base_noise = simplex3d(vec3<f32>(
        lon * params.base_frequency,
        lat * params.base_frequency,
        0.0
    ));
    let base_height = base_noise * params.base_amplitude;

    let detail_noise = simplex3d(vec3<f32>(
        lon * params.detail_frequency,
        lat * params.detail_frequency,
        100.0  // Offset in 3rd dimension for different pattern
    ));
    let detail_height = detail_noise * params.detail_amplitude;

    // Total height offset from base sphere radius
    let height = base_height + detail_height;

    // Project to sphere surface with height variation
    let r = params.planet_radius + height;
    let x = r * cos(lat) * cos(lon);
    let y = r * sin(lat);
    let z = r * cos(lat) * sin(lon);

    // UV coordinates for coloring
    let u = f32(grid_x) / f32(params.grid_size - 1);
    let v = f32(grid_z) / f32(params.grid_size - 1);

    // Write to vertex buffer
    vertices[idx].position = vec3<f32>(x, y, z);
    vertices[idx].uv = vec2<f32>(u, v);
}
