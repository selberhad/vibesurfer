# WGSL Compute Patterns

## Noise Functions on GPU

### Hash-Based Value Noise

Deterministic pseudorandom function for GPU (no texture lookups, pure arithmetic):

```wgsl
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p3_dot = dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3_dot);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);  // Grid cell
    let f = fract(p);  // Position in cell

    // Cubic smoothstep (C1 continuous)
    let u = f * f * (3.0 - 2.0 * f);

    // Bilinear interpolation of grid corners
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
```

**Properties**:
- ~10-15 ALU instructions (fast)
- Visually random (no obvious grid patterns)
- Not cryptographically secure (sufficient for visuals)

**Smoothstep curve**: `f(t) = 3t² - 2t³`
- Derivative is 0 at t=0 and t=1 (smooth cell boundaries)
- Upgrade to quintic `6t⁵ - 15t⁴ + 10t³` for C2 continuity if normal discontinuities visible

**Usage**:
```wgsl
let height = noise(position.xz * frequency) * amplitude;
let animated = noise(position.xz * frequency + vec2<f32>(time)) * amplitude;
```

### Multi-Octave Noise (Fractal)

Sum multiple frequencies for multi-scale detail:

```wgsl
fn fbm(p: vec2<f32>, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;

    for (var i = 0u; i < octaves; i++) {
        value += noise(p * frequency) * amplitude;
        amplitude *= 0.5;   // Each octave half as strong
        frequency *= 2.0;   // Each octave twice as detailed
    }
    return value;
}
```

**Performance**: Linear with octaves (2 octaves = 2× cost)

**Evidence**: toy4 validated 2-layer approach (base + detail) at 120 FPS

### Gradient Noise (Perlin/Simplex)

Interpolates random gradients (directions) instead of values.

**Why better than value noise**:
- Less blocky artifacts (gradient interpolation smoother)
- More organic look (directional flow)
- Better for natural terrain

**Simplified gradient noise**:
```wgsl
fn gradient(hash: vec2<f32>) -> vec2<f32> {
    // Convert hash to angle
    let angle = hash.x * 6.28318;
    return vec2<f32>(cos(angle), sin(angle));
}

fn gradient_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);  // Smoothstep

    // Get gradients at grid corners
    let g00 = gradient(hash_vec2(i));
    let g10 = gradient(hash_vec2(i + vec2<f32>(1.0, 0.0)));
    let g01 = gradient(hash_vec2(i + vec2<f32>(0.0, 1.0)));
    let g11 = gradient(hash_vec2(i + vec2<f32>(1.0, 1.0)));

    // Dot products with distance vectors
    let d00 = dot(g00, f);
    let d10 = dot(g10, f - vec2<f32>(1.0, 0.0));
    let d01 = dot(g01, f - vec2<f32>(0.0, 1.0));
    let d11 = dot(g11, f - vec2<f32>(1.0, 1.0));

    // Bilinear interpolation
    return mix(mix(d00, d10, u.x), mix(d01, d11, u.x), u.y);
}
```

**Trade-off**: More complex (~2× instructions) but higher quality

**When to use**: Upgrade from value noise if visual quality insufficient

---

## Frequency Tuning for Spherical Surfaces

Calculate noise frequency analytically to match desired feature spacing:

```
frequency = 1.0 / (desired_spacing_m / planet_radius)
```

**Example** (toy4):
- Planet radius: 1,000,000m
- Desired spacing: 75m hills
- Frequency: `1.0 / (75 / 1000000) = 13,333`

**Why**: Noise function operates in [0, 1] parameter space. Frequency maps world coordinates to that space at desired density.

**Two-layer approach**:
```wgsl
let base_freq = 13333.0;    // 75m spacing
let detail_freq = 50000.0;  // 20m spacing
let height = noise(lat_lon * base_freq) * 10.0 + noise(lat_lon * detail_freq) * 3.0;
```

**Global consistency**: Sample noise at unwrapped world coordinates (not torus/chunk-local) for seamless boundaries.

---

## Buffer Access Optimization

### Array-of-Structures vs Structure-of-Arrays

**AoS** (current pattern):
```wgsl
struct Vertex {
    position: vec3<f32>,
    uv: vec2<f32>,
    _padding: vec2<f32>,  // 32 bytes total
}
@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;

// Update only position.y
vertices[idx].position.y = new_height;  // Reads/writes full 32 bytes
```

**SoA** (alternative):
```wgsl
@group(0) @binding(0) var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read> uvs: array<vec2<f32>>;

positions[idx].y = new_height;  // Reads/writes only 16 bytes (vec3 padded to vec4)
```

**Trade-off**:
- **AoS**: Simpler (single buffer, easy render pipeline integration), wastes bandwidth
- **SoA**: Better bandwidth (only touch needed data), more complex (multiple buffers/bindings)

**Decision**: Use AoS unless profiling shows memory bandwidth bottleneck

### Read-Only vs Read-Write Storage Buffers

```wgsl
@group(0) @binding(0) var<storage, read_write> output: array<Vertex>;
@group(0) @binding(1) var<storage, read> input: array<f32>;
```

**Performance**: `read` allows aggressive GPU caching (no coherency overhead), `read_write` requires cache invalidation between threads.

**Best practice**: Use `read` when buffer won't be modified in this pass

---

## Common Gotchas

**Noise periodicity**: Hash may repeat at large coordinates (float precision limits). If camera travels >10,000m, hash aliasing possible.

**Time wraparound**: Unbounded `time` exceeds f32 precision after ~16M seconds (194 days). Wrap modulo or use frame count.

**Frequency aliasing**: If noise frequency too high (features smaller than grid spacing), temporal aliasing (shimmering). Limit to Nyquist (grid spacing / 2).

**Amplitude overflow**: `base + detail` can exceed range if not tuned. Clamp final height or tune amplitudes conservatively.

**Smoothstep discontinuity**: Cubic has continuous first derivative but discontinuous second derivative. Visible as subtle ridges in lighting. Upgrade to quintic if needed.

---

## Pattern: Hybrid CPU/GPU Noise

**Strategy** (from toy implementations):
- **Base terrain** (large, slow waves): High-quality noise (OpenSimplex) on CPU, cached
  - Recompute only on wrap/dirty (toroidal grid)
  - ~1% vertices update per frame
- **Detail layer** (small, fast ripples): Simplified GPU noise, every frame
  - All vertices updated in <1ms (massive parallelism)

**Implementation**:
```wgsl
let base = base_heights[idx];  // CPU-computed, read-only
let detail = noise(world_pos.xz * detail_freq + vec2<f32>(time)) * detail_amp;
vertices[idx].position.y = base + detail;
```

**Why**: Best of both - CPU quality for slow features, GPU speed for fast features

---

## Compute Patterns Beyond Embarrassingly Parallel

### Reduction (Sum/Max/Min)

Combine many inputs into fewer outputs using shared memory and barriers.

**Workgroup-level sum**:
```wgsl
var<workgroup> shared_sums: array<f32, 256>;

@compute @workgroup_size(256)
fn sum_reduce(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_index) local_idx: u32
) {
    // Load into shared memory
    shared_sums[local_idx] = data[global_id.x];
    workgroupBarrier();

    // Tree reduction (log2(N) steps)
    for (var stride = 128u; stride > 0u; stride /= 2u) {
        if (local_idx < stride) {
            shared_sums[local_idx] += shared_sums[local_idx + stride];
        }
        workgroupBarrier();
    }

    // Thread 0 writes workgroup result
    if (local_idx == 0u) {
        results[global_id.x / 256u] = shared_sums[0];
    }
}
```

**Use cases**: Average wave height, max displacement (LOD), histogram

**Performance**: Barrier synchronization expensive (~10-100 cycles), minimize barrier count

### Stencil Operations (Neighbor Access)

Each output depends on neighbors in grid (blur, edge detection, cellular automata).

**3×3 blur pattern**:
```wgsl
@compute @workgroup_size(16, 16)  // 2D workgroup for spatial locality
fn blur(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= width || y >= height) { return; }

    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let nx = clamp(i32(x) + dx, 0, i32(width) - 1);
            let ny = clamp(i32(y) + dy, 0, i32(height) - 1);
            sum += input[ny * width + nx];
        }
    }
    output[y * width + x] = sum / 9.0;
}
```

**Optimization**: Use shared memory to cache tile of input (reduce global memory reads)

**Use cases**: FFT synthesis (frequency domain → spatial domain), flowfields

### 2D Dispatch for Spatial Data

**Pattern**:
```rust
let workgroup_size_x = 16;
let workgroup_size_y = 16;
let workgroup_count_x = (width + workgroup_size_x - 1) / workgroup_size_x;
let workgroup_count_y = (height + workgroup_size_y - 1) / workgroup_size_y;
compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
```

**Benefit**: Adjacent threads access spatially adjacent data (better cache locality)

---

## References

- [Book of Shaders - Noise](https://thebookofshaders.com/11/)
- Toy implementations: `toys/toy3_infinite_camera`, `toys/toy4_spherical_chunks`
- Ken Perlin - [Improving Noise](https://mrl.nyu.edu/~perlin/paper445.pdf) (quintic smoothstep, gradient noise)
