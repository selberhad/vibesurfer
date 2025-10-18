# WGSL Compute Patterns and Techniques

**Purpose**: Common compute shader patterns, noise functions on GPU, buffer access optimization for ocean mesh deformation

**Audience**: AI agents implementing procedural systems (ocean waves, flowfields, particle systems)

---

## Compute Pattern Taxonomy

### Pattern 1: Embarrassingly Parallel (Ocean Mesh)

**Definition**: Each output element depends only on input at same index (or read-only data)

**Characteristics**:
- No inter-thread communication needed
- No shared memory required
- Scales linearly with thread count
- Simplest to implement, hardest to break

**Ocean mesh example**:
```wgsl
@compute @workgroup_size(256)
fn update_ocean(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&vertices)) { return; }

    // Read base (constant across frame)
    let base = base_heights[idx];

    // Compute detail (function of position only, not other vertices)
    let detail = noise(vertices[idx].position.xz * frequency);

    // Write to unique location
    vertices[idx].position.y = base + detail;
}
```

**Key trait**: `vertices[N]` only writes to index `N`, never reads from or writes to other indices

**Performance**: Near-perfect GPU utilization (no synchronization overhead)

---

### Pattern 2: Reduction (Sum/Max/Min)

**Definition**: Combine many inputs into fewer outputs (e.g., sum array of 1M floats → single total)

**Characteristics**:
- Requires shared memory within workgroup
- Needs barrier synchronization
- Multi-pass for large arrays (workgroup-level reduction, then CPU or second pass)
- Classic parallel algorithm challenge

**Workgroup-level sum**:
```wgsl
var<workgroup> shared_sums: array<f32, 256>;

@compute @workgroup_size(256)
fn sum_reduce(@builtin(global_invocation_id) global_id: vec3<u32>,
              @builtin(local_invocation_index) local_idx: u32) {
    // Step 1: Load into shared memory
    shared_sums[local_idx] = data[global_id.x];
    workgroupBarrier();  // Wait for all threads to load

    // Step 2: Tree reduction (log2(N) steps)
    for (var stride = 128u; stride > 0u; stride /= 2u) {
        if (local_idx < stride) {
            shared_sums[local_idx] += shared_sums[local_idx + stride];
        }
        workgroupBarrier();  // Sync after each reduction step
    }

    // Step 3: Thread 0 writes workgroup result
    if (local_idx == 0u) {
        results[global_id.x / 256u] = shared_sums[0];
    }
}
```

**Gotcha**: Barrier synchronization is expensive (~10-100 cycles). Minimize barrier count.

**Ocean mesh usage**: Not needed for vertex updates (embarrassingly parallel), but could use for:
- Computing average wave height (debugging)
- Finding max displacement (LOD decisions)
- Histogram of vertex heights (analytics)

---

### Pattern 3: Prefix Sum (Scan)

**Definition**: Cumulative sum - output[i] = sum of input[0..i]

**Example**: `[1, 2, 3, 4]` → `[1, 3, 6, 10]`

**Use cases**:
- Allocate variable-length output (count elements, then prefix sum for offsets)
- Stream compaction (filter sparse array into dense)
- Radix sort
- Particle emission (allocate slots for new particles)

**Complexity**: Requires work-efficient algorithm (Blelloch scan), multiple passes, barriers

**Defer**: Not needed for ocean mesh. Study when implementing particle systems.

---

### Pattern 4: Stencil Operations (Neighbor Access)

**Definition**: Each output depends on neighbors in grid (e.g., blur, edge detection, cellular automata)

**Characteristics**:
- 2D/3D spatial patterns
- Can use shared memory to cache tile of input (reduce global memory reads)
- Coalesced access critical (adjacent threads read adjacent data)

**3x3 blur example** (simplified):
```wgsl
@compute @workgroup_size(16, 16)  // 2D workgroup for 2D grid
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

**Ocean mesh**: Not current pattern (each vertex independent), but future FFT synthesis might use stencil-like patterns (frequency domain → spatial domain requires neighbor relationships)

---

## Noise Functions on GPU

### CPU vs GPU Implementation Trade-offs

| Consideration | CPU (Rust) | GPU (WGSL) |
|---------------|------------|------------|
| Library support | `noise` crate (OpenSimplex) | No libraries - implement from scratch |
| Precision | f64 common | f32 only (f64 optional, slow) |
| Quality | High (complex gradient tables) | Simplified (hash-based pseudorandom) |
| Performance | ~10-100µs for 1M samples | ~0.5-1ms for 1M samples (massively parallel) |
| Consistency | Bit-exact across runs | Bit-exact if hash deterministic |

**Ocean mesh strategy**:
- **Base terrain** (slow-changing large waves): OpenSimplex on CPU, cache per vertex
- **Detail layer** (fast-changing ripples): Simplified GPU noise, recompute every frame

**Why not OpenSimplex on GPU?**
- Requires gradient table lookup (256+ vec2 entries)
- Texture fetch overhead or large uniform buffer
- Hash-based noise simpler, fewer memory accesses

---

### Hash-Based Noise (GPU-Friendly)

**Concept**: Deterministic pseudorandom function from vec2 → f32

**Requirements**:
1. Deterministic (same input → same output)
2. Uniform distribution (~0.0-1.0 range)
3. No visible patterns (spatial coherence but not periodic artifacts)
4. Fast (cheap arithmetic, no memory fetches)

**Hash function** (from our `ocean_compute.wgsl`):
```wgsl
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let p3_dot = dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3_dot);
}
```

**How it works**:
1. `fract(p * 0.1031)` - Map input to [0, 1), multiply by irrational to spread values
2. `dot(p3, shuffled_p3 + offset)` - Mix components via dot product (cheap ALU ops)
3. `fract((p3.x + p3.y) * p3_dot)` - Final mixing, output in [0, 1)

**Key properties**:
- Pure arithmetic (no texture lookups, no buffer reads)
- ~10-15 ALU instructions (fast)
- Visually random (no obvious grid patterns)

**Trade-off**: Not cryptographically secure, may have subtle correlations, but sufficient for visual noise

---

### Value Noise (Bilinear Interpolation)

**Concept**: Hash integer grid points, interpolate between them for smooth variation

**Algorithm**:
```wgsl
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);  // Integer part (grid cell)
    let f = fract(p);  // Fractional part (position in cell)

    // Smoothstep interpolation (cubic Hermite curve)
    let u = f * f * (3.0 - 2.0 * f);

    // Hash four corners of grid cell
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));

    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
```

**Smoothstep curve**: `f(t) = 3t² - 2t³`
- Derivative is 0 at t=0 and t=1 (smooth transition between cells)
- Better than linear interpolation (avoids visible grid)
- Cheaper than quintic (Perlin's improved noise uses `6t⁵ - 15t⁴ + 10t³`)

**Frequency/amplitude**:
```wgsl
let noise_value = noise(position * frequency) * amplitude;
```
- `frequency` - Scale input (higher = more detail, smaller features)
- `amplitude` - Scale output (wave height)

**Ocean mesh usage**:
```wgsl
let noise_pos = vec2<f32>(x_world, z_world) * params.detail_frequency;
let detail_noise = noise(noise_pos + vec2<f32>(params.time));
let detail_height = detail_noise * params.detail_amplitude;
```

**Time offset**: `noise(pos + time)` shifts the noise field (animated ripples)

---

### Gradient Noise (Perlin/Simplex)

**Concept**: Interpolate random gradients (directions) instead of values

**Why better?**
- Less blocky artifacts (gradient interpolation smoother than value interpolation)
- More organic look (directional flow)

**Why we're not using it (yet)**:
- More complex implementation (~2x code)
- Requires gradient table or more complex hash (vec2 → vec2, not vec2 → f32)
- Value noise "good enough" for detail layer (base terrain handles large waves)

**Future consideration**: If detail layer looks too blocky at certain frequencies, upgrade to gradient noise

---

### Octave Layering (Fractal Noise)

**Concept**: Sum multiple noise octaves at different frequencies/amplitudes

**Pattern**:
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

**Result**: Multi-scale detail (large waves + medium ripples + fine texture)

**Performance cost**: Linear with octaves (4 octaves = 4× noise() calls)

**Ocean mesh**: Currently single-octave (base on CPU + detail on GPU). Could add 2-3 detail octaves on GPU for richer surface.

---

## Buffer Access Patterns

### Coalesced Access (CRITICAL for Performance)

**Principle**: Adjacent threads should access adjacent memory addresses

**Why it matters**: GPU memory controller fetches cache lines (128 bytes typical)
- Thread 0 reads `data[0]`, thread 1 reads `data[1]`, ..., thread 31 reads `data[31]`
- GPU coalesces into 1-2 cache line fetches (128 bytes = 32 × f32)
- **Result**: 32× fewer memory transactions

**Bad pattern** (strided access):
```wgsl
let idx = global_id.x * 256;  // Thread 0 → data[0], thread 1 → data[256], ...
let value = data[idx];  // NO coalescing - 32 separate cache line fetches
```

**Good pattern** (sequential access):
```wgsl
let idx = global_id.x;  // Thread 0 → data[0], thread 1 → data[1], ...
let value = data[idx];  // Coalesced - 1-2 cache line fetches
```

**Ocean mesh**: Sequential access (`global_id.x` maps directly to vertex index) → optimal coalescing

---

### Structure of Arrays (SoA) vs Array of Structures (AoS)

**AoS** (our current Vertex layout):
```wgsl
struct Vertex {
    position: vec3<f32>,  // 12 bytes
    uv: vec2<f32>,        // 8 bytes
    // Padded to 32 bytes (GPU alignment)
}
@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
```

**Access pattern**:
```wgsl
vertices[idx].position.y = new_height;
```
- Reads 32 bytes (entire Vertex), modifies 4 bytes, writes back 32 bytes
- Wastes bandwidth (reading uv when only updating position.y)

**SoA** (alternative):
```wgsl
@group(0) @binding(0) var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read> uvs: array<vec2<f32>>;
```

**Access pattern**:
```wgsl
positions[idx].y = new_height;
```
- Reads 16 bytes (vec3 padded to vec4), modifies 4 bytes, writes back 16 bytes
- Better bandwidth (only touch position data)

**Trade-off**:
- **AoS pro**: Simpler data layout, easier to pass to render pipeline (single buffer)
- **AoS con**: Wastes bandwidth if only updating subset of fields
- **SoA pro**: Bandwidth-efficient for partial updates
- **SoA con**: More buffers, more bind group entries, more complex code

**Ocean mesh decision**: Stick with AoS (simplicity > bandwidth for now). Vertex update is ~1ms, not bottleneck. Refactor to SoA only if profiling shows memory bandwidth limited.

---

### Read-Only vs Read-Write Storage Buffers

**Syntax**:
```wgsl
@group(0) @binding(0) var<storage, read_write> output: array<Vertex>;
@group(0) @binding(1) var<storage, read> input: array<f32>;
```

**Performance implication**:
- `read` - GPU can aggressively cache (no invalidation needed)
- `read_write` - GPU must ensure coherency (cache invalidation between threads)

**Ocean mesh**:
- `vertices` - `read_write` (update position.y in place)
- `base_heights` - `read` (constant during compute pass, cache-friendly)
- `params` - `uniform` (even faster path than storage buffers)

**Best practice**: Use `read` when buffer won't be modified in this pass (helps GPU optimizer)

---

## Open Questions (Post-Priority 1)

1. **Noise quality**: Does hash-based noise produce visible artifacts at certain frequencies? (Need visual inspection during integration)
2. **Octave layering cost**: How many detail octaves fit in frame budget? (Profile: 1 vs 2 vs 3 octaves)
3. **Gradient noise benefit**: Would Perlin/Simplex look noticeably better for ocean surface? (Subjective - needs A/B comparison)
4. **SoA performance**: Would SoA layout measurably improve bandwidth? (Profile with GPU tools - may not matter if compute-bound, not memory-bound)
5. **Shared memory benefit**: Could we tile vertex updates to exploit shared memory? (Unlikely - embarrassingly parallel pattern doesn't benefit from shared memory)

---

## WGSL Storage Buffer Alignment (CRITICAL)

### The Problem: Silent Buffer Overflows

**Symptom**: Compute shader only processes first N elements of array, remaining elements untouched

**Example**: 100-vertex grid, only 75 vertices computed, remaining 25 at (0,0,0) or garbage

**Root cause**: WGSL storage buffer arrays require structs padded to **next 16-byte multiple**

### Alignment Rules

**Rule 1: Individual field alignment**
```wgsl
vec3<f32>  // 12 bytes data, but requires 16-byte alignment
vec2<f32>  // 8 bytes data, 8-byte alignment
f32        // 4 bytes data, 4-byte alignment
```

**Rule 2: Array element alignment** (THE CRITICAL ONE)
```wgsl
array<MyStruct>  // Each element must start at 16-byte multiple
```

**Example that FAILS**:
```rust
// Rust side
#[repr(C)]
struct Vertex {
    position: [f32; 3],  // 12 bytes
    _padding1: f32,      // 4 bytes → 16 bytes total
    uv: [f32; 2],        // 8 bytes
    // Total: 24 bytes
}
```

```wgsl
// WGSL side
struct Vertex {
    position: vec3<f32>,  // 12 bytes, 16-byte aligned
    _padding1: f32,       // 4 bytes
    uv: vec2<f32>,        // 8 bytes
    // Total: 24 bytes in Rust, BUT...
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
```

**What actually happens**:
- Rust allocates buffer: `100 vertices × 24 bytes = 2400 bytes`
- WGSL expects: `vertices[N]` at offset `N × 32` (next 16-byte multiple after 24)
- WGSL writes `vertices[75]` at byte 2400 → **buffer overflow!**
- Vertices 76-99 never written (or write out of bounds)

### The Fix: Pad to 32 Bytes

```rust
// Rust side - CORRECT
#[repr(C)]
struct Vertex {
    position: [f32; 3],   // 12 bytes
    _padding1: f32,       // 4 bytes → 16 bytes
    uv: [f32; 2],         // 8 bytes
    _padding2: [f32; 2],  // 8 bytes → 32 bytes total
}
// Now: 100 vertices × 32 bytes = 3200 bytes
```

```wgsl
// WGSL side - CORRECT
struct Vertex {
    position: vec3<f32>,
    _padding1: f32,
    uv: vec2<f32>,
    _padding2: vec2<f32>,  // Explicit padding to 32 bytes
}

@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
// Now WGSL and Rust agree: vertices[N] at offset N × 32
```

### Debugging Tips

**Symptom 1**: Only first ~75% of array processed
- Check: `struct_size_in_rust × 1.33 ≈ struct_size_wgsl_expects`
- Example: 24 bytes × 1.33 = 32 bytes

**Symptom 2**: Vertex data corrupted (wrong positions)
- Individual fields misaligned
- Check: `offset_in_rust == offset_in_wgsl`
- Use debug buffer readback to inspect raw bytes

**Prevention**:
1. Always pad structs to next 16-byte multiple when used in storage buffer arrays
2. Use `std::mem::size_of::<T>()` to verify Rust size matches expected WGSL size
3. Write headless test that reads back ALL array elements and verifies sentinel values

**Reference**: [WGSL Spec - Storage Class Layout](https://www.w3.org/TR/WGSL/#storage-class)

---

## Gotchas

**Gotcha 1: Noise periodicity**
- Hash-based noise may repeat at large coordinates (float precision limits)
- If camera travels far (x > 10,000), hash(p) may alias
- **Solution**: Use double-precision coordinates for noise input (cast to f32 only for final value)

**Gotcha 2: Time wraparound**
- `params.time` grows unbounded (eventually exceeds f32 precision)
- After ~16,777,216 seconds (194 days), time increments become coarse
- **Solution**: Wrap time modulo some period, or use frame count modulo

**Gotcha 3: Frequency aliasing**
- If `detail_frequency` too high, noise features smaller than grid spacing
- **Result**: Temporal aliasing (shimmering, moire patterns)
- **Solution**: Limit frequency to Nyquist (grid spacing / 2)

**Gotcha 4: Amplitude overflow**
- `base_height + detail_height` can exceed expected range
- If not clamped, vertices may shoot infinitely high
- **Solution**: Clamp final height or tune amplitudes to sum < max

**Gotcha 5: Smoothstep discontinuity**
- Cubic smoothstep has continuous first derivative, but second derivative discontinuous at cell boundaries
- Visible as subtle ridges in lighting (normal vector has kinks)
- **Solution**: Upgrade to quintic smoothstep (`6t⁵ - 15t⁴ + 10t³`) for C2 continuity

---

## Pattern from Vibesurfer Codebase

**Hybrid CPU/GPU approach** (from HANDOFF.md):
- **Base terrain** (large, slow waves): OpenSimplex on CPU, cached in `base_heights[]`
  - Recompute only when vertex wraps (toroidal grid)
  - ~1% of vertices update per frame (cheap CPU cost)
- **Detail layer** (small, fast ripples): GPU hash-based noise, computed every frame
  - All 1M vertices updated in ~0.5ms (massive parallelism)

**Why this split?**
- CPU: High-quality noise (OpenSimplex), infrequent updates (cache-friendly)
- GPU: Simplified noise (hash-based), frequent updates (parallel-friendly)
- Best of both: Quality base + fast detail

**Implementation**:
```wgsl
let base = base_heights[idx];  // CPU-computed, read-only, cached
let detail = noise(...);       // GPU-computed, per-frame
vertices[idx].position.y = base + detail;  // Combine
```

---

## References

**Primary sources**:
- [Book of Shaders - Noise](https://thebookofshaders.com/11/) (cached: `.webcache/wgsl/noise_patterns.html`)
- `src/ocean_compute.wgsl` - Our GPU noise implementation
- `src/noise.rs` - Our CPU noise (OpenSimplex wrapper)
- [WebGPU Fundamentals - Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)

**Further reading** (not cached):
- Ken Perlin - [Improving Noise](https://mrl.nyu.edu/~perlin/paper445.pdf) (quintic smoothstep, gradient noise)
- Inigo Quilez - [Value Noise](https://iquilezles.org/articles/morenoise/) (detailed analysis)
- GPU Gems - [Chapter 5: Implementing Improved Perlin Noise](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-5-implementing-improved-perlin-noise)

---

**Created**: 2025-10-17 (Research mode, Priority 1)
**Last updated**: 2025-10-17
