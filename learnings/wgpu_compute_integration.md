# wgpu Compute Pipeline Integration (Rust)

## Resource Creation Flow

Order matters (dependencies):

```
1. Device + Queue (from adapter)
2. ShaderModule (WGSL source)
3. Buffers (storage, uniform, vertex)
4. BindGroupLayout (describes bindings)
5. BindGroup (binds buffers to layout)
6. PipelineLayout (references layouts)
7. ComputePipeline (layout + shader)
8. CommandEncoder → ComputePass → dispatch
9. Queue::submit(commands)
```

**Principle**: Layouts are blueprints, bind groups are instances

---

## Buffer Creation

### Usage Flags

```rust
wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
```

**Common combinations**:
- Compute output + render input: `VERTEX | STORAGE | COPY_DST`
- Compute read-only input: `STORAGE | COPY_DST`
- Per-frame constants: `UNIFORM | COPY_DST`

**Key constraint**: Can't combine `STORAGE` with `MAP_READ`/`MAP_WRITE` (GPU-only buffers)

### Initialization

**With data**:
```rust
use wgpu::util::DeviceExt;

let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Vertex Buffer"),
    contents: bytemuck::cast_slice(&vertices),
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
});
```

**Without data** (output-only):
```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    size: vertex_count * std::mem::size_of::<Vertex>() as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
    mapped_at_creation: false,
});
```

### Updates

**Preferred** (simple, async):
```rust
queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
```

**Note**: Always copies entire buffer. For large partial updates, use staging buffer + `copy_buffer_to_buffer()`.

---

## Bind Group Layout

Defines shader interface (like function signature):

```rust
let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    entries: &[
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
        // ... more bindings
    ],
});
```

**Fields**:
- `binding` - Matches `@binding(N)` in WGSL
- `visibility` - COMPUTE, VERTEX, FRAGMENT, or combinations
- `read_only` - `false` for `read_write`, `true` for `read`

---

## Bind Group

Binds actual buffers to layout (like function arguments):

```rust
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &bind_group_layout,
    entries: &[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: vertex_buffer.as_entire_binding(),
        },
        // ... more buffers
    ],
});
```

---

## Compute Pipeline

```rust
let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("Compute Pipeline"),
    layout: Some(&pipeline_layout),
    module: &compute_shader,
    entry_point: Some("main"),  // Must match WGSL @compute fn name
    compilation_options: Default::default(),
    cache: None,
});
```

**Shader loading**:
```rust
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
});
```

**Error**: `create_shader_module()` panics on invalid WGSL. Use `RUST_LOG=wgpu=warn` during development.

---

## Dispatch Execution

```rust
pub fn dispatch_compute(&self, params: &Params, element_count: u32) {
    // Update uniforms
    self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[*params]));

    // Create command encoder
    let mut encoder = self.device.create_command_encoder(&Default::default());

    {
        let mut compute_pass = encoder.begin_compute_pass(&Default::default());
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);

        // Dispatch workgroups (round up to cover all elements)
        let workgroup_size = 256;
        let workgroup_count = (element_count + workgroup_size - 1) / workgroup_size;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }  // Compute pass ends here

    self.queue.submit(std::iter::once(encoder.finish()));
}
```

**Dispatch calculation**: Round-up division ensures all elements covered, shader bounds-checks overshoot

---

## Synchronization

### Compute → Render

**Same encoder** (implicit barrier):
```rust
let mut encoder = device.create_command_encoder(&Default::default());

// Compute pass
{ let mut pass = encoder.begin_compute_pass(&Default::default()); /* ... */ }

// Render pass (compute results visible)
{ let mut pass = encoder.begin_render_pass(&Default::default()); /* ... */ }

queue.submit(Some(encoder.finish()));
```

**Separate submits** (also works, slightly more overhead):
```rust
// Submit 1: Compute
queue.submit(Some(compute_encoder.finish()));

// Submit 2: Render (results visible)
queue.submit(Some(render_encoder.finish()));
```

### CPU ← GPU (Reading Results)

Copy to mappable buffer:
```rust
encoder.copy_buffer_to_buffer(&storage_buffer, 0, &read_buffer, 0, size);
queue.submit(Some(encoder.finish()));

read_buffer.slice(..).map_async(wgpu::MapMode::READ, |_| {});
device.poll(wgpu::Maintain::Wait);

{
    let data = read_buffer.slice(..).get_mapped_range();
    let typed: &[f32] = bytemuck::cast_slice(&data);
    // ... use data
}

read_buffer.unmap();
```

**When needed**: Validation tests, debugging (not normal render loop)

---

## Performance Profiling

### Timestamp Queries

Measure GPU time for compute passes:

**Setup** (requires feature flag):
```rust
let (device, queue) = adapter.request_device(
    &wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TIMESTAMP_QUERY,
        // ...
    },
    None,
).await?;

let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
    label: Some("Timestamp Queries"),
    count: 2,
    ty: wgpu::QueryType::Timestamp,
});
```

**Usage**:
```rust
let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
        query_set: &query_set,
        beginning_of_pass_write_index: Some(0),
        end_of_pass_write_index: Some(1),
    }),
    // ...
});
```

**Read results**: Copy query buffer to mappable buffer, compute delta

**When to use**: Performance debugging, validating optimizations

---

## Struct Alignment (bytemuck)

Rust structs passed to GPU must be `Pod` + `Zeroable`:

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Params {
    pub camera_pos: Vec3,      // 12 bytes
    pub amplitude: f32,        // 4 bytes → 16 bytes
    pub frequency: f32,        // 4 bytes → 20 bytes
    pub _padding: [f32; 3],    // 12 bytes → 32 bytes (multiple of 16) ✓
}
```

**Rule**: Uniform buffer struct size must be multiple of 16 bytes

**Validation**:
```rust
assert_eq!(std::mem::size_of::<Params>(), 32);
```

---

## Common Errors

**Shader compilation**: Invalid WGSL → panic. Use `RUST_LOG=wgpu=debug` to see errors.

**Buffer size mismatch**: Binding buffer smaller than shader expects → validation warning or GPU error.

**Bounds overflow**: Dispatch too many threads without bounds check → out-of-bounds access. Always check in shader:
```wgsl
if (idx >= arrayLength(&data)) { return; }
```

**Bind group stale**: Creating bind group after modifying buffer doesn't update binding. Must recreate bind group or use dynamic offset.

---

## Multiple Bind Groups Pattern

Separate per-frame from per-object data:

```rust
// Layout with 2 bind groups
let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    bind_group_layouts: &[&per_frame_layout, &per_object_layout],
    // ...
});

// In shader
@group(0) @binding(0) var<uniform> per_frame: PerFrameData;
@group(1) @binding(0) var<uniform> per_object: PerObjectData;

// Set bind groups
compute_pass.set_bind_group(0, &per_frame_bind_group, &[]);
compute_pass.set_bind_group(1, &per_object_bind_group, &[]);
```

**Benefit**: Update only changed data (e.g., per-frame every frame, per-object on change)

---

## Gotchas

**Encoder must finish before next frame**: Can't reuse encoder, must call `finish()` and `submit()` each frame

**Buffer writes not immediate**: `write_buffer()` stages write, executed on next submit

**Pod/Zeroable restrictions**: Can't derive on types with references, `String`, `Vec`, etc. Only primitives and arrays.

**Shader entry point case-sensitive**: `entry_point: Some("Main")` ≠ `@compute fn main()`

---

## References

- [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/)
- [Learn wgpu - Compute](https://sotrh.github.io/learn-wgpu/)
- Toy implementations: `toys/toy3_infinite_camera`, `toys/toy4_spherical_chunks`
