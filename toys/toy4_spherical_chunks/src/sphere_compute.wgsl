// Compute shader: Project flat grid to sphere surface

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
    _padding1: f32,
    _padding2: f32,
    _padding3: f32,
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<uniform> params: SphereParams;

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

    // Project to sphere surface (flat sphere, no height variation)
    let r = params.planet_radius;
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
