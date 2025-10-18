# GPU Compute Shader Fundamentals (WGSL)

**Purpose**: Foundational knowledge for GPU compute programming - execution model, memory hierarchy, WGSL syntax essentials

**Audience**: AI agents implementing ocean mesh deformation and future procedural systems

---

## Mental Model: Thread Hierarchy

**Key insight**: GPU compute is massively parallel. You don't write one function that loops - you write one function that runs thousands of times in parallel.

**Hierarchy** (smallest to largest):
1. **Invocation** (thread) - Single execution of compute shader entry point
2. **Workgroup** - Small collection of threads (e.g., 256) that can cooperate
3. **Dispatch** - Grid of workgroups launched by one `dispatch_workgroups()` call

**Example**:
```wgsl
@compute @workgroup_size(256)  // 256 threads per workgroup
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;  // Unique thread ID across entire dispatch
    data[idx] = data[idx] * 2.0;  // Each thread processes one element
}
```

If you call `dispatch_workgroups(4, 1, 1)`:
- 4 workgroups × 256 threads = 1024 total invocations
- Each invocation runs `main()` with different `global_invocation_id`

---

## WGSL Syntax Essentials

### Entry Point
```wgsl
@compute @workgroup_size(X, Y, Z)  // Defines threads per workgroup
fn entry_name(@builtin(...) param: type) {
    // Shader logic
}
```

**Constraints**:
- Workgroup size: `X * Y * Z ≤ 256` (default limit `maxComputeInvocationsPerWorkgroup`)
- Individual dimensions: X ≤ 256, Y ≤ 256, Z ≤ 64
- 1D workgroups common for linear data: `@workgroup_size(256)` ≡ `@workgroup_size(256, 1, 1)`

### Variable Bindings

**Syntax**: `@group(N) @binding(M) var<address_space, access> name: type;`

**Address spaces**:
- `storage` - GPU memory, large buffers (read-only or read-write)
- `uniform` - GPU memory, small constant data (≤64KB typical), read-only, faster access
- `workgroup` - Shared within workgroup, fast, small (16KB-32KB typical)
- `private` - Thread-local, registers/cache

**Access modes**:
- `storage, read` - Read-only storage buffer
- `storage, read_write` - Writable storage buffer
- Uniforms always read-only (no access specifier needed)

**Example**:
```wgsl
@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read> base_data: array<f32>;
@group(0) @binding(2) var<uniform> params: MyParams;
```

**Key constraint**: Can't use `read_write` storage buffers in vertex/fragment shaders (compute only)

### Built-in Thread Indices

**`global_invocation_id`** - Unique 3D ID for this thread across entire dispatch
- Formula: `workgroup_id * workgroup_size + local_invocation_id`
- **Most commonly used** for indexing into buffers

**`local_invocation_id`** - 3D ID within current workgroup (0..workgroup_size-1)
- Use for indexing shared memory within workgroup

**`workgroup_id`** - 3D ID of this workgroup in dispatch grid
- All threads in workgroup have same `workgroup_id`

**`num_workgroups`** - Dimensions passed to `dispatch_workgroups(x, y, z)`

**`local_invocation_index`** - Linearized thread index within workgroup
- Formula: `id.x + id.y * size.x + id.z * size.x * size.y`
- Useful for 1D indexing into workgroup-shared arrays

**Ocean mesh pattern**:
```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;  // Linear index for 1D vertex array

    if (idx >= arrayLength(&vertices)) {  // Bounds check crucial
        return;  // Workgroup may overshoot array size
    }

    vertices[idx].position.y = compute_height(idx);
}
```

**Why bounds check?** If array has 1M elements and workgroup is 256:
- Dispatch: `(1000000 + 255) / 256 = 3907` workgroups
- Total invocations: `3907 * 256 = 1,000,192`
- Last 192 threads access out-of-bounds → must early-return

---

## Memory Model

### GPU Memory Hierarchy (Fastest to Slowest)

1. **Registers** (private address space) - Per-thread variables
2. **Workgroup shared memory** (workgroup address space) - ~16-32KB, shared within workgroup
3. **L1/L2 cache** - Automatic, exploited via coalesced memory access
4. **VRAM** (storage/uniform buffers) - Large, slower

**Key optimization**: Threads in a workgroup accessing adjacent memory addresses → coalesced into fewer cache lines → faster

**Example** (good coalescing):
```wgsl
// Thread 0 reads vertices[0], thread 1 reads vertices[1], etc.
// GPU coalesces into few cache line fetches
let vertex = vertices[global_id.x];
```

**Example** (poor coalescing):
```wgsl
// Thread 0 reads vertices[0], thread 1 reads vertices[256], etc.
// GPU can't coalesce - many random accesses
let vertex = vertices[global_id.x * 256];  // Strided access - slow
```

### Storage Buffer vs Uniform Buffer

| Feature | Storage Buffer | Uniform Buffer |
|---------|---------------|----------------|
| Size limit | Large (≥128MB typical) | Small (64KB typical) |
| Access | Read or read-write | Read-only |
| Speed | Slower (cached) | Faster (dedicated path) |
| Use case | Vertex data, large arrays | Per-frame constants |

**Ocean mesh usage**:
- `vertices` - Storage buffer (1M vertices × 32 bytes = 32MB, read-write)
- `base_heights` - Storage buffer (1M floats = 4MB, read-only)
- `params` - Uniform buffer (camera pos, amplitude, frequency = <64 bytes)

### Alignment Requirements

**Rule**: Uniform buffer members must align to 16-byte boundaries for structs

**Example**:
```wgsl
struct ComputeParams {
    camera_pos: vec3<f32>,      // 12 bytes (3 × f32)
    detail_amplitude: f32,      // 4 bytes
    detail_frequency: f32,      // 4 bytes
    time: f32,                  // 4 bytes
    _padding: vec2<f32>,        // 8 bytes - aligns to 16-byte multiple
}
// Total: 32 bytes (aligned to 16)
```

**Why padding?** GPU memory controllers expect aligned access. Misaligned reads → slower or error.

**Note**: `vec3<u32>` in arrays pads to 16 bytes (not 12), always account for padding when calculating buffer sizes.

---

## Workgroup Sizing Strategy

**Question**: Why `@workgroup_size(256)` in our ocean shader?

**Answer**: GPU hardware runs threads in lockstep groups (SIMD). Typical GPU:
- Runs 16-64 threads simultaneously (same instruction)
- Multiple groups per workgroup
- 64-256 threads per workgroup is sweet spot

**WebGPU recommendation**: Default to 64 unless profiling shows otherwise
- Below 64: May underutilize GPU parallelism
- Above 256: May hit hardware limits (slower fallback)
- 256: Conservative choice for "probably near-optimal"

**M1 GPU specifics** (our platform):
- Apple Silicon GPU has 128-thread SIMD width (varies by model)
- 256 threads = 2 SIMD groups → good occupancy
- Could benchmark 64, 128, 256 to find optimum

**Dispatch calculation**:
```rust
let vertex_count = 1_048_576;  // 1024×1024 grid
let workgroup_size = 256;
let workgroup_count = (vertex_count + workgroup_size - 1) / workgroup_size;
// workgroup_count = 4096
pass.dispatch_workgroups(workgroup_count, 1, 1);
```

**Result**: 4096 workgroups × 256 threads = 1,048,576 invocations (matches vertex count exactly, last workgroup partially idle)

---

## Synchronization and Race Conditions

**Key constraint**: Threads run in parallel, no ordering guarantees

**Race condition example** (BAD):
```wgsl
@group(0) @binding(0) var<storage, read_write> result: array<f32>;

@compute @workgroup_size(32)
fn bad_example(@builtin(local_invocation_id) id: vec3<u32>) {
    result[0] = f32(id.x);  // All 32 threads write to same location
    // Final value? Undefined (whoever writes last wins)
}
```

**Safe pattern** (GOOD - embarrassingly parallel):
```wgsl
@compute @workgroup_size(256)
fn safe_example(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    result[idx] = compute_value(idx);  // Each thread writes to unique location
    // No races - independent writes
}
```

**Ocean mesh**: Embarrassingly parallel (each vertex independent)
- Thread N updates `vertices[N]`, never touches `vertices[M]` where M ≠ N
- No synchronization needed within compute pass

**Barriers** (when needed for shared memory):
```wgsl
var<workgroup> shared_data: array<f32, 256>;

@compute @workgroup_size(256)
fn with_barrier(@builtin(local_invocation_index) idx: u32) {
    shared_data[idx] = load_data(idx);
    workgroupBarrier();  // Wait for all threads to finish writing
    let value = shared_data[(idx + 1) % 256];  // Safe to read neighbor's data
}
```

**Constraint**: Barriers only synchronize within workgroup, not across workgroups
- Can't assume workgroup A finishes before workgroup B
- Can't synchronize across entire dispatch within one shader

---

## Common Patterns

### Pattern 1: Array Processing (Ocean Mesh)
```wgsl
@compute @workgroup_size(256)
fn process_array(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&data)) { return; }
    data[idx] = transform(data[idx]);
}
```
**Use**: Independent operations on array elements

### Pattern 2: Grid Processing (2D/3D)
```wgsl
@compute @workgroup_size(16, 16)  // 256 threads (16×16)
fn process_grid(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= width || y >= height) { return; }
    let idx = y * width + x;
    grid[idx] = compute_cell(x, y);
}
```
**Use**: 2D grids (images, heightmaps) - spatial locality benefits

### Pattern 3: Reduction (Sum/Max/Min)
*Deferred to Priority 1 (requires shared memory + barriers)*

---

## Open Questions (Post-Priority 0)

1. **Workgroup sizing**: Is 256 optimal for M1 GPU? (Needs profiling: 64 vs 128 vs 256)
2. **Memory alignment**: Does padding affect performance or just correctness? (Cache line boundaries?)
3. **Dispatch overhead**: What's the CPU cost of `dispatch_workgroups()`? (Negligible vs compute time?)
4. **Bounds checking cost**: Does early-return for out-of-bounds threads hurt performance? (Warp divergence?)
5. **Struct packing**: Can we pack `Vertex` more efficiently? (Currently `vec3<f32>` + `vec2<f32>` = 20 bytes, padded to 32?)

---

## Gotchas

**Gotcha 1: Workgroup size is compile-time constant**
- Can't parameterize `@workgroup_size(N)` from Rust
- Must use template string or preprocessor if varying size needed

**Gotcha 2: vec3 padding in arrays**
- `array<vec3<u32>>` - each element takes 16 bytes (not 12)
- Account for padding when calculating buffer sizes

**Gotcha 3: Storage buffers unmappable**
- Can't directly read storage buffer from CPU
- Must copy to `MAP_READ` buffer via `copy_buffer_to_buffer()`

**Gotcha 4: No printf debugging**
- Can't print from shader
- Debug via writing to debug buffer or tools (RenderDoc, Metal debugger)

**Gotcha 5: Integer division truncates**
- `5 / 2 = 2` (not 2.5) for integer types
- Use `f32` for fractional math

---

## References

**Primary sources**:
- [WebGPU Fundamentals - Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html) (cached: `.webcache/wgsl/webgpu_compute_fundamentals.html`)
- Our spike: `src/ocean_compute.wgsl`

**Further reading** (not cached yet):
- [WGSL Specification](https://www.w3.org/TR/WGSL/) - Language reference
- [WebGPU Specification](https://www.w3.org/TR/webgpu/) - API surface
- [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/) - Rust bindings

---

**Created**: 2025-10-17 (Research mode, Priority 0)
**Last updated**: 2025-10-17
