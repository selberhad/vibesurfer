# wgpu Compute Pipeline Integration (Rust)

**Purpose**: Practical guide to creating compute pipelines in wgpu - buffers, bind groups, pipeline creation, dispatch, synchronization

**Audience**: AI agents integrating GPU compute into Vibesurfer rendering system

---

## Mental Model: wgpu Resource Creation Flow

**Order matters**: Some resources depend on others

```
1. Device + Queue (from adapter)
   ↓
2. ShaderModule (from WGSL source)
   ↓
3. Buffers (storage, uniform, vertex)
   ↓
4. BindGroupLayout (describes bindings)
   ↓
5. BindGroup (binds buffers to layout)
   ↓
6. PipelineLayout (references bind group layouts)
   ↓
7. ComputePipeline (references pipeline layout + shader)
   ↓
8. CommandEncoder → ComputePass → dispatch
   ↓
9. Queue::submit(commands)
```

**Key principle**: Layouts are blueprints, bind groups are instances

---

## Buffer Creation

### Buffer Usage Flags

**Syntax**:
```rust
wgpu::BufferUsages::FLAG1 | wgpu::BufferUsages::FLAG2
```

**Common flags**:
- `VERTEX` - Can be bound as vertex buffer in render pass
- `INDEX` - Can be bound as index buffer in render pass
- `UNIFORM` - Can be bound as uniform buffer (small, fast)
- `STORAGE` - Can be bound as storage buffer (large, read or read-write)
- `COPY_SRC` - Can be source of copy operation
- `COPY_DST` - Can be destination of copy operation (for `queue.write_buffer()`)
- `MAP_READ` - Can be mapped for CPU reading
- `MAP_WRITE` - Can be mapped for CPU writing

**Ocean mesh compute requirements**:
- `vertices`: `VERTEX | STORAGE | COPY_DST` (render + compute read-write + CPU update)
- `base_heights`: `STORAGE | COPY_DST` (compute read-only + CPU update)
- `params`: `UNIFORM | COPY_DST` (compute read-only + CPU update per frame)

**Key constraint**: Can't combine `STORAGE` with `MAP_READ`/`MAP_WRITE`
- Storage buffers live on GPU, can't be directly mapped to CPU
- To read results: copy to separate buffer with `MAP_READ` flag

---

### Buffer Initialization

**Method 1: create_buffer_init** (from `wgpu::util::DeviceExt`):
```rust
use wgpu::util::DeviceExt;

let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Vertex Buffer"),
    contents: bytemuck::cast_slice(&vertices),  // Initial data
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
});
```

**Method 2: create_buffer** (uninitialized):
```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("Vertex Buffer"),
    size: vertex_count * std::mem::size_of::<Vertex>() as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
    mapped_at_creation: false,  // Don't map immediately
});
```

**When to use each**:
- `create_buffer_init`: Have initial data (vertex buffer, index buffer)
- `create_buffer`: Will fill later or used as output-only (compute results)

**Ocean mesh**: Use `create_buffer_init` (vertices + base_heights populated from CPU)

---

### Buffer Updates

**Method 1: queue.write_buffer()** (preferred):
```rust
queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
```

**Pros**:
- Simple API
- Async (doesn't block CPU)
- GPU manages transfer

**Cons**:
- Always copies entire buffer (even if only one field changed)

**Method 2: Mapped buffer** (for large partial updates):
```rust
let mut encoder = device.create_command_encoder(&Default::default());
let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Staging Buffer"),
    contents: &updated_data,
    usage: wgpu::BufferUsages::COPY_SRC,
});
encoder.copy_buffer_to_buffer(&staging_buffer, 0, &target_buffer, offset, size);
queue.submit(Some(encoder.finish()));
```

**Ocean mesh pattern**:
- **Uniform params**: `write_buffer()` every frame (tiny, 32 bytes)
- **Base heights**: `write_buffer()` only for dirty indices (sparse updates)
  - Could optimize: copy only changed region if contiguous
  - For now: full buffer update when any vertex wraps (simple)

---

## Shader Module Creation

**Syntax**:
```rust
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Ocean Compute Shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("ocean_compute.wgsl").into()),
});
```

**include_str!** macro:
- Embeds file contents as `&'static str` at compile time
- Path relative to current source file
- Shader code validated at runtime (not compile time)

**Error handling**:
- Invalid WGSL → panic on `create_shader_module()` (can't catch easily)
- Use validation layers during dev (env var `RUST_LOG=wgpu=debug`)

**Ocean mesh**: Load `src/ocean_compute.wgsl`

---

## Bind Group Layout

**Purpose**: Defines interface between shader and buffers (like function signature)

**Syntax**:
```rust
let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("Compute Bind Group Layout"),
    entries: &[
        // Binding 0: vertices (storage, read_write)
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Binding 1: base_heights (storage, read-only)
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        // Binding 2: params (uniform)
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
});
```

**Field meanings**:
- `binding` - Matches `@binding(N)` in WGSL
- `visibility` - Which shader stages can access (COMPUTE, VERTEX, FRAGMENT, or combinations)
- `ty` - Buffer type (Storage/Uniform/Texture/Sampler)
- `read_only` - `true` for `var<storage, read>`, `false` for `var<storage, read_write>`
- `has_dynamic_offset` - Allow offset at bind time (advanced, usually false)
- `min_binding_size` - Validate buffer size (None = no validation)
- `count` - Array of bindings (None = single binding)

**Ocean mesh layout** (3 bindings):
1. Vertices (storage read-write)
2. Base heights (storage read-only)
3. Params (uniform)

---

## Bind Group

**Purpose**: Binds actual buffers to layout slots (like function call arguments)

**Syntax**:
```rust
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("Compute Bind Group"),
    layout: &bind_group_layout,
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: vertex_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 1,
            resource: base_heights_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: params_buffer.as_entire_binding(),
        },
    ],
});
```

**`as_entire_binding()`**: Binds full buffer (most common)

**Alternative** (bind subrange):
```rust
resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
    buffer: &my_buffer,
    offset: 256,
    size: Some(std::num::NonZeroU64::new(1024).unwrap()),
})
```

**When to use subrange**: Large buffer containing multiple datasets, bind different regions per draw call

**Ocean mesh**: Bind entire buffers (one dataset per buffer)

---

## Pipeline Layout

**Purpose**: Defines bind group slots for pipeline (can have multiple bind groups)

**Syntax**:
```rust
let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("Compute Pipeline Layout"),
    bind_group_layouts: &[&compute_bind_group_layout],
    push_constant_ranges: &[],
});
```

**Multiple bind groups**:
```rust
bind_group_layouts: &[&per_frame_layout, &per_object_layout],
```

**Usage in shader**:
```wgsl
@group(0) @binding(0) var<uniform> per_frame: PerFrameData;
@group(1) @binding(0) var<uniform> per_object: PerObjectData;
```

**Ocean mesh**: Single bind group (group 0) with 3 bindings

**Push constants**: Small data passed directly (not via buffer) - not used in ocean mesh

---

## Compute Pipeline

**Purpose**: Combines shader entry point + layout into executable pipeline

**Syntax**:
```rust
let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("Ocean Compute Pipeline"),
    layout: Some(&pipeline_layout),
    module: &compute_shader,
    entry_point: Some("main"),  // Matches `@compute fn main() { ... }` in WGSL
    compilation_options: Default::default(),
    cache: None,
});
```

**`entry_point`**: Must match `@compute` function name in shader

**`compilation_options`**: Advanced (define constants, optimization hints) - usually default

**`cache`**: Pipeline caching (speed up creation on subsequent runs) - optional

**Ocean mesh**: Entry point `"main"` from `ocean_compute.wgsl`

---

## Compute Pass Execution

**Pattern**:
```rust
pub fn dispatch_ocean_compute(&self, params: &ComputeParams, vertex_count: u32) {
    // Update uniform buffer
    self.queue.write_buffer(
        &self.compute_params_buffer,
        0,
        bytemuck::cast_slice(&[*params])
    );

    // Create command encoder
    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder"),
    });

    // Begin compute pass
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Ocean Compute Pass"),
            timestamp_writes: None,
        });

        // Set pipeline and bind groups
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

        // Dispatch workgroups
        let workgroup_size = 256;
        let workgroup_count = (vertex_count + workgroup_size - 1) / workgroup_size;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }  // Compute pass drops here (ends recording)

    // Submit command buffer
    self.queue.submit(std::iter::once(encoder.finish()));
}
```

**Key points**:
1. **Update uniforms before encoding** (via `write_buffer()`)
2. **Encoder scoping** - compute pass must drop before `encoder.finish()`
3. **Workgroup calculation** - round up division to cover all elements
4. **set_bind_group(0, ...)** - `0` matches `@group(0)` in shader

---

## Dispatch Calculation

**Formula** (1D dispatch):
```rust
let workgroup_count = (element_count + workgroup_size - 1) / workgroup_size;
compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
```

**Why round up**: Ensure all elements covered, bounds-check in shader handles overshoot

**Example**:
- 1,048,576 vertices (1024×1024)
- Workgroup size: 256
- Calculation: `(1048576 + 255) / 256 = 4096`
- Total invocations: `4096 * 256 = 1,048,576` (exact match)

**Example with overshoot**:
- 1,000,000 vertices
- Workgroup size: 256
- Calculation: `(1000000 + 255) / 256 = 3907`
- Total invocations: `3907 * 256 = 1,000,192`
- Overshoot: 192 threads (must bounds-check)

**2D dispatch** (for grid):
```rust
let workgroup_size_x = 16;
let workgroup_size_y = 16;
let workgroup_count_x = (width + workgroup_size_x - 1) / workgroup_size_x;
let workgroup_count_y = (height + workgroup_size_y - 1) / workgroup_size_y;
compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
```

**Ocean mesh**: 1D dispatch (linear vertex array)

---

## Synchronization

### Compute → Render

**Question**: When are compute results visible to render pass?

**Answer**: Implicit barrier between command buffer submissions

**Pattern 1: Same encoder** (compute before render):
```rust
let mut encoder = device.create_command_encoder(&Default::default());

// Compute pass
{
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());
    // ... dispatch compute
}

// Render pass (implicit barrier - compute results visible)
{
    let mut render_pass = encoder.begin_render_pass(&Default::default());
    // ... draw using compute-updated buffers
}

queue.submit(Some(encoder.finish()));
```

**Pattern 2: Separate encoders** (compute in one submit, render in another):
```rust
// Submit 1: Compute
let mut compute_encoder = device.create_command_encoder(&Default::default());
{
    let mut compute_pass = compute_encoder.begin_compute_pass(&Default::default());
    // ... dispatch compute
}
queue.submit(Some(compute_encoder.finish()));

// Submit 2: Render (barrier between submits - compute results visible)
let mut render_encoder = device.create_command_encoder(&Default::default());
{
    let mut render_pass = render_encoder.begin_render_pass(&Default::default());
    // ... draw
}
queue.submit(Some(render_encoder.finish()));
```

**Ocean mesh**: Separate submits (compute in `dispatch_ocean_compute()`, render in `render()`)
- Compute updates `vertex_buffer`
- Render pass reads `vertex_buffer` as `VERTEX` binding
- GPU ensures coherency (write in compute visible to read in render)

---

### CPU ← GPU (Reading Results)

**Pattern**: Copy to mappable buffer, then map
```rust
// 1. Copy storage buffer to map-readable buffer
encoder.copy_buffer_to_buffer(
    &storage_buffer,
    0,
    &read_buffer,
    0,
    size
);
queue.submit(Some(encoder.finish()));

// 2. Map buffer async
read_buffer.slice(..).map_async(wgpu::MapMode::READ, |result| {
    if let Ok(()) = result {
        // Mapping ready
    }
});

// 3. Poll device (required to process async operations)
device.poll(wgpu::Maintain::Wait);

// 4. Get mapped range
{
    let data = read_buffer.slice(..).get_mapped_range();
    let typed_data: &[f32] = bytemuck::cast_slice(&data);
    // ... read typed_data
}

// 5. Unmap before reuse
read_buffer.unmap();
```

**Ocean mesh**: Not needed (compute results used by GPU render, not read by CPU)

**Debugging use case**: Validate compute output matches CPU reference

---

## Struct Alignment (bytemuck)

**Requirement**: Rust structs passed to GPU must be `Pod` + `Zeroable`

**Derive macro**:
```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ComputeParams {
    pub camera_pos: Vec3,          // 12 bytes (3 × f32)
    pub detail_amplitude: f32,     // 4 bytes
    pub detail_frequency: f32,     // 4 bytes
    pub time: f32,                 // 4 bytes
    pub _padding: Vec2,            // 8 bytes (force 32-byte total)
}
```

**`#[repr(C)]`**: C-style layout (no Rust reordering)

**Alignment rules** (uniform buffers):
- Struct size must be multiple of 16 bytes
- `vec3` in structs acts like `vec4` (16-byte aligned)

**Checking alignment**:
```rust
assert_eq!(std::mem::size_of::<ComputeParams>(), 32);  // Must be multiple of 16
```

**Common mistake**: Forgetting padding → GPU reads wrong offsets → garbage data

---

## Error Handling

### Shader Compilation Errors

**Problem**: `create_shader_module()` panics on invalid WGSL

**Detection**:
```bash
RUST_LOG=wgpu=warn cargo run
```

**Example error**:
```
wgpu::Device: Shader module creation failed:
    Parsing error at line 42: expected ';', found 'let'
```

**Fix**: Check WGSL syntax, validate types match bindings

---

### Buffer Size Mismatch

**Problem**: Binding buffer smaller than shader expects

**Example**:
```wgsl
@group(0) @binding(0) var<uniform> data: array<f32, 1000>;  // Expects 4000 bytes
```

```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size: 1000,  // Only 1000 bytes → mismatch!
    // ...
});
```

**Detection**: Validation layer warning or runtime error (GPU-dependent)

**Fix**: Ensure buffer size ≥ shader expectation

**Ocean mesh**: Dynamic array (`array<Vertex>`) → no fixed size requirement

---

### Bounds Overflow

**Problem**: Dispatch more threads than array size, no bounds check in shader

**Example**:
```wgsl
@compute @workgroup_size(256)
fn bad(@builtin(global_invocation_id) id: vec3<u32>) {
    data[id.x] = 0.0;  // No bounds check - crashes if id.x >= arrayLength(&data)
}
```

**Fix**: Early return on bounds check
```wgsl
if (idx >= arrayLength(&data)) { return; }
```

**Ocean mesh**: Bounds check present in `ocean_compute.wgsl:46`

---

## Performance Profiling

### Timestamp Queries

**Setup** (requires `TIMESTAMP_QUERY` feature):
```rust
let (device, queue) = adapter.request_device(
    &wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TIMESTAMP_QUERY,
        // ...
    },
    None,
).await?;
```

**Usage**:
```rust
let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
    label: Some("Timestamp Queries"),
    count: 2,
    ty: wgpu::QueryType::Timestamp,
});

let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
        query_set: &query_set,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: Some(1),
    }),
    // ...
});
```

**Read results**: Copy query buffer to mappable buffer, read timestamps

**Ocean mesh**: Defer until profiling needed (adds complexity)

---

## Integration Checklist (Ocean Mesh)

**Files to modify**:
1. `src/rendering.rs` - Add compute pipeline + buffers (~100 lines)
2. `src/main.rs` - Call `dispatch_ocean_compute()` instead of CPU update (~5 lines)
3. `src/ocean/mesh.rs` - Track dirty base indices (~5 lines)

**New resources in RenderSystem**:
- `compute_shader: wgpu::ShaderModule`
- `compute_pipeline: wgpu::ComputePipeline`
- `compute_bind_group: wgpu::BindGroup`
- `base_heights_buffer: wgpu::Buffer`
- `compute_params_buffer: wgpu::Buffer`

**Buffer usage changes**:
- `vertex_buffer`: Add `STORAGE` flag (was `VERTEX | COPY_DST`, now `VERTEX | STORAGE | COPY_DST`)

**New method**:
- `dispatch_ocean_compute(&self, params: &ComputeParams, vertex_count: u32)`

**Call site** (main.rs):
```rust
// Before: CPU update
// ocean.update(camera_pos, time, ...);
// render_system.update_vertices(&ocean.grid.vertices);

// After: GPU compute
let params = ComputeParams { camera_pos, detail_amplitude, detail_frequency, time, _padding: Vec2::ZERO };
render_system.dispatch_ocean_compute(&params, ocean.grid.vertices.len() as u32);
```

---

## Open Questions (Post-Priority 2)

1. **Pipeline cache**: Worth implementing for faster startup? (Measure: time to `create_compute_pipeline()`)
2. **Async compilation**: Can we create pipelines async while showing loading screen? (Not critical for single pipeline)
3. **Multiple bind groups**: Should we split per-frame params (group 0) from static buffers (group 1)? (Premature optimization - single group fine)
4. **Dynamic offsets**: Could we use one large buffer with offsets instead of multiple small buffers? (More complex API, no clear benefit)
5. **Error recovery**: How to gracefully handle shader compilation failure? (Currently panics - could show error screen)

---

## Gotchas

**Gotcha 1: Encoder must finish before next frame**
- Can't reuse encoder across frames
- Must call `encoder.finish()` and `queue.submit()` each frame

**Gotcha 2: Bind group persists pipeline state**
- Creating bind group after modifying buffer doesn't update binding
- Must create new bind group or use dynamic offset to rebind

**Gotcha 3: Buffer writes not immediate**
- `queue.write_buffer()` stages write, executed on next submit
- Don't read buffer from CPU immediately after write (not flushed yet)

**Gotcha 4: Pod/Zeroable requires all fields safe**
- Can't derive on types with references, `String`, `Vec`, etc.
- Only primitive types and arrays of primitives

**Gotcha 5: Shader entry point case-sensitive**
- `entry_point: Some("Main")` ≠ `@compute fn main()`
- Must match exactly (including capitalization)

---

## Pattern from Vibesurfer Codebase

**Render system owns all GPU resources**:
- Device, queue, pipelines, buffers in `RenderSystem` struct
- Encapsulates GPU state (main.rs doesn't touch wgpu directly)

**Separation of concerns**:
- `OceanGrid` (CPU data structure) - vertices, indices, base heights
- `RenderSystem` (GPU resources) - buffers, pipelines, shaders
- `main.rs` (orchestration) - update ocean, dispatch compute, render

**Update flow** (proposed):
1. `main.rs`: Call `ocean.update_wrapping()` (updates only wrapped vertices' base heights)
2. `main.rs`: Call `render_system.upload_dirty_base_heights()` (sparse CPU→GPU transfer)
3. `main.rs`: Call `render_system.dispatch_ocean_compute()` (GPU updates all vertices)
4. `main.rs`: Call `render_system.render()` (draw using updated vertices)

**Benefit**: Clear ownership, testable components (can profile each step)

---

## References

**Primary sources**:
- `src/rendering.rs` - Existing render pipeline setup
- HANDOFF.md - Integration plan with code snippets
- [wgpu examples - compute](https://github.com/gfx-rs/wgpu/tree/trunk/examples/src/hello_compute) (not cached, reference)

**Further reading**:
- [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/)
- [Learn wgpu - Compute](https://sotrh.github.io/learn-wgpu/) (tutorial series)
- [WebGPU Fundamentals - Compute](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html) (JavaScript but concepts map to Rust)

---

**Created**: 2025-10-17 (Research mode, Priority 2)
**Last updated**: 2025-10-17
