# GPU Compute Fundamentals (WGSL)

## Core Mental Model

**GPU compute = thousands of identical functions running in parallel**

You write one function that runs N times simultaneously, not one function with a loop.

**Hierarchy**: Invocation (thread) → Workgroup (e.g., 256 threads) → Dispatch (grid of workgroups)

**Example**:
```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;  // Unique ID across entire dispatch
    data[idx] = data[idx] * 2.0;  // Each thread processes one element
}
```

Dispatch `(4, 1, 1)` → 4 workgroups × 256 threads = 1024 invocations

---

## Critical Constraints

### Workgroup Size Limits
- Total: `X * Y * Z ≤ 256` (WebGPU default)
- Per-dimension: X ≤ 256, Y ≤ 256, Z ≤ 64
- **Validated choice**: 256 (M1 GPU = 128-thread SIMD → 256 = 2 groups)
- Evidence: toy3/toy4 sustained 120 FPS with 256 workgroup size

### Memory Alignment (CRITICAL)

**Uniform buffers**: Struct size must be multiple of 16 bytes
```rust
#[repr(C)]
struct Params {
    camera_pos: Vec3,     // 12 bytes
    amplitude: f32,       // 4 bytes
    _padding: Vec2,       // 8 bytes → 24 bytes total (NOT 16-byte aligned!)
    _padding2: f32,       // 4 bytes → 28 bytes
    _padding3: f32,       // 4 bytes → 32 bytes ✓ (multiple of 16)
}
```

**Storage buffer arrays**: Element size must be multiple of 16 bytes
```rust
struct Vertex {
    position: [f32; 3],  // 12 bytes
    _padding1: f32,      // 4 bytes → 16 bytes
    uv: [f32; 2],        // 8 bytes → 24 bytes (NOT 16-byte aligned!)
    _padding2: [f32; 2], // 8 bytes → 32 bytes ✓
}
```

**Evidence**: toy3 TerrainParams required 80 bytes (52 data + 28 padding) after multiple iterations. Misalignment causes GPU to read wrong offsets or silently process subset of array.

### Bounds Checking Required

Round-up dispatch overshoots array size:
```wgsl
@compute @workgroup_size(256)
fn process(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&data)) { return; }  // Critical: prevent out-of-bounds
    data[idx] = transform(data[idx]);
}
```

Example: 1M elements → `(1000000 + 255) / 256 = 3907` workgroups → 1,000,192 invocations (192 overshoot)

---

## WGSL Syntax Essentials

### Variable Bindings
```wgsl
@group(0) @binding(0) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read> base_data: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;
```

**Address spaces**:
- `storage` - Large buffers (read or read_write), GPU memory
- `uniform` - Small constants (≤64KB), faster access, read-only
- `workgroup` - Shared within workgroup (16-32KB), fast

**Key constraint**: `read_write` storage buffers only in compute shaders (not vertex/fragment)

### Built-in Indices
- `global_invocation_id` - **Most common**: unique 3D ID across dispatch, use for buffer indexing
- `local_invocation_id` - 3D ID within workgroup (for shared memory)
- `workgroup_id` - Which workgroup this is in dispatch grid

---

## Memory Access Patterns

### Coalesced Access (Performance Critical)

**Good** (adjacent threads → adjacent memory):
```wgsl
let idx = global_id.x;  // Thread 0→data[0], thread 1→data[1], ...
let value = data[idx];  // GPU coalesces into 1-2 cache line fetches
```

**Bad** (strided access):
```wgsl
let idx = global_id.x * 256;  // Thread 0→data[0], thread 1→data[256], ...
let value = data[idx];  // 32 separate cache line fetches (slow!)
```

### Storage vs Uniform Buffers

| Feature | Storage | Uniform |
|---------|---------|---------|
| Size limit | ≥128MB | 64KB |
| Access | Read or read-write | Read-only |
| Speed | Slower (cached) | Faster |
| Use case | Vertex data, large arrays | Per-frame constants |

---

## Common Gotchas

**vec3 padding in arrays**: `array<vec3<u32>>` elements take 16 bytes (not 12)

**Storage buffers unmappable**: Can't read directly from CPU, must copy to `MAP_READ` buffer

**No printf debugging**: Write to debug buffer or use GPU debugger (RenderDoc, Metal)

**Workgroup size is compile-time**: Can't parameterize `@workgroup_size(N)` from Rust

**Integer division truncates**: `5 / 2 = 2` for integers, use `f32` for fractions

---

## Compute Pattern: Embarrassingly Parallel

Most common pattern for vertex processing (ocean mesh, terrain):

```wgsl
@compute @workgroup_size(256)
fn update(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&vertices)) { return; }

    // Each thread writes to unique location (no races)
    vertices[idx].position.y = compute_height(idx);
}
```

**Characteristics**:
- No inter-thread communication
- No shared memory needed
- Scales linearly with thread count
- Near-perfect GPU utilization

---

## Synchronization Primitives

### Workgroup Barriers

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

**Cost**: ~10-100 cycles, minimize use

### Race Condition Example

**Bad** (race):
```wgsl
@compute @workgroup_size(32)
fn bad(@builtin(local_invocation_id) id: vec3<u32>) {
    result[0] = f32(id.x);  // All 32 threads write to same location
    // Final value undefined (whoever writes last wins)
}
```

**Good** (no race):
```wgsl
@compute @workgroup_size(256)
fn good(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    result[idx] = compute_value(idx);  // Each thread writes unique location
}
```

---

## References

- [WebGPU Fundamentals - Compute](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)
- [WGSL Spec](https://www.w3.org/TR/WGSL/)
- Toy implementations: `toys/toy3_infinite_camera`, `toys/toy4_spherical_chunks`
