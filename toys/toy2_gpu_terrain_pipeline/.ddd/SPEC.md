# Toy Model 2: GPU Terrain Pipeline Specification

Validate GPU compute shader terrain generation performance and integration with rendering pipeline.

## Overview

**What it does:** Generates large-scale procedural terrain (1024×1024 vertices) using GPU compute shaders, renders it as wireframe, and measures per-frame performance to validate this approach can achieve 60+ FPS.

**Key principles:**
- GPU compute as single source of truth for terrain geometry
- Per-frame regeneration with varying parameters (simulated audio reactivity)
- Compute → render pipeline integration (no CPU readback)
- Realistic grid size (1024×1024 = 1,048,576 vertices)
- Minimal complexity: single noise octave, no toroidal wrapping

**Scope:** Isolates two complexity axes:
1. GPU compute shader terrain generation performance at production scale
2. Compute-to-render pipeline integration pattern in wgpu

**Integration context:**
- Input: Simulated audio bands (3 floats updated per frame)
- Output: Rendered wireframe terrain, FPS metrics
- No physics integration, no camera animation, no audio synthesis

## Data Model

### TerrainParams (Uniform Buffer)

```rust
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TerrainParams {
    // Terrain generation
    base_amplitude: f32,       // meters (e.g., 150.0)
    base_frequency: f32,       // cycles/meter (e.g., 0.003)
    detail_amplitude: f32,     // audio-modulated, meters (e.g., 2.0 + bass * 3.0)
    detail_frequency: f32,     // audio-modulated (e.g., 0.1 + mid * 0.15)

    // Grid properties
    grid_size: u32,            // vertices per side (1024)
    grid_spacing: f32,         // meters between vertices (2.0)

    // Animation
    time: f32,                 // seconds (for detail layer animation)
    seed: u32,                 // noise seed
}
```

### Vertex (Storage Buffer, Read-Write)

```rust
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],  // World-space XYZ
    uv: [f32; 2],        // Texture coordinates (unused, for future)
}
```

### AudioBands (Simulated)

```rust
struct AudioBands {
    low: f32,   // 0.0-10.0 range
    mid: f32,   // 0.0-10.0 range
    high: f32,  // 0.0-10.0 range
}
```

## Core Operations

### Operation 1: Initialize Terrain Pipeline

**Syntax:**
```rust
let pipeline = TerrainPipeline::new(device, queue, config);
```

**Parameters:**
- `device: &wgpu::Device` - GPU device
- `queue: &wgpu::Queue` - Command queue
- `config: TerrainConfig` - Grid size, spacing, initial parameters

**Behavior:**
1. Creates compute shader pipeline from `terrain_compute.wgsl`
2. Creates render shader pipeline from `terrain_render.wgsl`
3. Allocates vertex buffer (STORAGE | VERTEX, size = grid_size² vertices)
4. Allocates index buffer (for wireframe triangles)
5. Allocates uniform buffer (UNIFORM | COPY_DST, size = TerrainParams)
6. Creates bind groups for compute and render passes

**Validation:**
- Shader compilation succeeds (no WGSL errors)
- Buffer sizes match grid_size² exactly
- Bind group layouts compatible with shaders

**Example:**
```rust
let config = TerrainConfig {
    grid_size: 1024,
    grid_spacing_m: 2.0,
    base_amplitude_m: 150.0,
    base_frequency: 0.003,
    detail_amplitude_m: 2.0,
    detail_frequency: 0.1,
};
let pipeline = TerrainPipeline::new(&device, &queue, config);
```

### Operation 2: Update Terrain (Per-Frame)

**Syntax:**
```rust
pipeline.update(time_s, audio_bands);
```

**Parameters:**
- `time_s: f32` - Elapsed time in seconds (for animation)
- `audio_bands: &AudioBands` - Simulated FFT bands (low/mid/high)

**Behavior:**
1. Computes modulated parameters:
   - `detail_amplitude = base + audio_bands.low * 3.0`
   - `detail_frequency = base + audio_bands.mid * 0.15`
2. Updates `TerrainParams` uniform buffer via `queue.write_buffer()`
3. Dispatches compute shader:
   - Workgroup size: 256 threads
   - Dispatch count: `(grid_size² + 255) / 256` workgroups
4. GPU writes vertex positions directly to storage buffer

**Validation:**
- Uniform buffer updated before dispatch
- Dispatch count covers all vertices (no out-of-bounds access)
- Compute shader completes before render pass

**Example:**
```rust
let audio_bands = AudioBands { low: 5.0, mid: 3.0, high: 2.0 };
pipeline.update(elapsed_time, &audio_bands);
```

### Operation 3: Render Terrain

**Syntax:**
```rust
pipeline.render(encoder, view);
```

**Parameters:**
- `encoder: &mut wgpu::CommandEncoder` - Command encoder for this frame
- `view: &wgpu::TextureView` - Render target

**Behavior:**
1. Creates render pass with `view` as color attachment
2. Sets render pipeline (wireframe, alpha blending)
3. Binds vertex buffer (same buffer written by compute shader)
4. Binds index buffer (static wireframe topology)
5. Draws indexed triangles

**Validation:**
- Render pass uses same vertex buffer as compute output
- No GPU errors or validation warnings
- Frame completes and presents

**Example:**
```rust
let mut encoder = device.create_command_encoder(&Default::default());
pipeline.render(&mut encoder, &surface_view);
queue.submit(Some(encoder.finish()));
```

### Operation 4: Measure Performance

**Syntax:**
```rust
let fps = pipeline.current_fps();
let frame_time_ms = pipeline.avg_frame_time_ms();
```

**Behavior:**
- Tracks frame timestamps using `Instant::now()`
- Computes rolling average over last 60 frames
- Returns FPS and average frame time

**Validation:**
- FPS computed as `1.0 / avg_frame_time`
- Metrics updated every frame

## Test Scenarios

### Simple: Static Terrain (No Animation)

**Setup:**
- Grid: 512×512 (smaller for fast validation)
- Audio bands: constant `AudioBands { low: 0.0, mid: 0.0, high: 0.0 }`
- Time: 0.0 (frozen)

**Expected:**
- Terrain renders with base parameters only
- FPS stable (should be >100 FPS at 512×512)
- Vertex positions deterministic (same seed → same output)

**Success criteria:**
- [ ] Terrain visible (not black screen or corrupted)
- [ ] FPS > 60 consistently
- [ ] Two runs with same seed produce identical first frame

### Complex: Animated, Audio-Reactive Terrain

**Setup:**
- Grid: 1024×1024 (production scale)
- Audio bands: sine wave modulation
  ```rust
  AudioBands {
      low: 5.0 + 5.0 * (time * 0.5).sin(),
      mid: 3.0 + 2.0 * (time * 1.0).sin(),
      high: 2.0 + 1.0 * (time * 2.0).sin(),
  }
  ```
- Time: increments each frame

**Expected:**
- Terrain visibly animates (detail layer pulses)
- Amplitude changes visible (hills grow/shrink with bass)
- Frequency changes visible (choppiness varies with mids)
- FPS ≥ 60 maintained

**Success criteria:**
- [ ] Terrain animates smoothly (no stuttering)
- [ ] FPS ≥ 60 for 10 seconds at 1024×1024 grid
- [ ] Visual confirmation: amplitude scales with `audio_bands.low`
- [ ] Visual confirmation: frequency scales with `audio_bands.mid`

### Error: Invalid Grid Sizes

**Setup:**
- Grid sizes: 0, 1, 2049, u32::MAX

**Expected:**
- Grid 0: Panics or returns error at initialization
- Grid 1: Renders single vertex (degenerate but valid)
- Grid 2049: May fail (buffer size > GPU limit), graceful error
- Grid u32::MAX: Fails at buffer allocation (too large)

**Success criteria:**
- [ ] Invalid sizes caught at initialization (before shader dispatch)
- [ ] Error messages specify which limit was exceeded
- [ ] No GPU device lost errors (graceful failure)

### Performance: Scaling Validation

**Setup:**
- Run at multiple grid sizes: 256, 512, 1024, 2048 (if supported)
- Measure FPS for each

**Expected scaling:**
- 256×256 (65k vertices): >200 FPS
- 512×512 (262k vertices): >120 FPS
- 1024×1024 (1M vertices): ≥60 FPS (critical threshold)
- 2048×2048 (4M vertices): Best effort (may drop below 60)

**Success criteria:**
- [ ] FPS scaling approximately linear with vertex count
- [ ] 1024×1024 achieves ≥60 FPS consistently
- [ ] No GPU memory errors at production scale

### Integration: Toroidal Wrapping (Final De-Risking)

**Setup:**
- Static camera at origin
- Grid flows backward continuously (simulates camera forward motion)
- When vertices exit behind camera (z < camera_z - threshold), wrap to front
- Noise sampling uses world-space coordinates (not grid indices)

**Implementation:**
- Add `camera_pos: vec3<f32>` to TerrainParams uniform
- Compute shader calculates per-vertex world position from grid index + camera offset
- Apply toroidal wrapping logic: `if world_z < camera_z - wrap_threshold { world_z += grid_extent }`
- Sample noise at wrapped world coordinates

**Expected:**
- Terrain appears to scroll infinitely
- No seams or discontinuities at wrap boundary
- Terrain coherence maintained (same world position = same height)
- FPS unchanged from static test (wrapping is cheap)

**Success criteria:**
- [ ] Vertices wrap correctly (visual inspection: no gaps)
- [ ] Noise sampling coherent across wrap boundary (no visible seam)
- [ ] FPS ≥60 maintained with continuous wrapping
- [ ] World-space coordinates verified (same XZ = same height before/after wrap)

## Success Criteria

**Functional Requirements:**
- [ ] Compute shader compiles and dispatches without errors
- [ ] Vertex buffer written by compute, read by render (no corruption)
- [ ] Terrain renders as visible wireframe geometry
- [ ] Audio band parameters modulate amplitude and frequency correctly
- [ ] Animation (time parameter) produces smooth motion

**Performance Requirements:**
- [ ] 1024×1024 grid achieves ≥60 FPS on M1 Mac
- [ ] Frame time breakdown measured (compute vs render time)
- [ ] GPU utilization efficient (no obvious bottlenecks)

**Integration Requirements:**
- [ ] Single command encoder handles compute → render transition
- [ ] No CPU readback required (GPU-to-GPU workflow)
- [ ] Parameter updates via uniform buffer each frame

**Code Quality:**
- [ ] Compute shader reuses noise implementation from toy1 (Stefan Gustavson simplex)
- [ ] Render shader minimal (pass-through vertex positions)
- [ ] No unsafe code, no unwraps in hot path

**Validation:**
- [ ] Visual inspection confirms terrain looks correct (smooth hills, not corrupted)
- [ ] FPS display overlaid on window (live feedback)
- [ ] Deterministic: same seed produces same terrain

**Toroidal Wrapping (Phase 2):**
- [ ] Vertices wrap correctly when exiting camera view
- [ ] Noise sampling coherent across wrap boundaries (no seams)
- [ ] FPS ≥60 maintained with continuous wrapping
- [ ] World-space coordinate system working correctly

## Implementation Phases

**Phase 1: Static Terrain (Core Validation)**
- Build compute → render pipeline
- Validate performance at 1024×1024
- Prove audio-reactive parameter updates work
- **Exit criteria: ≥60 FPS achieved**

**Phase 2: Toroidal Wrapping (Integration De-Risking)**
- Add camera position tracking
- Implement GPU-side vertex wrapping logic
- Validate world-space noise coherence
- **Exit criteria: Infinite scrolling with no seams, FPS maintained**

## Out of Scope (Deferred to Main Integration)

- ❌ Camera animation (static camera sufficient for wrapping test)
- ❌ Real audio synthesis (simulated bands sufficient)
- ❌ Physics queries (CPU readback, separate concern)
- ❌ Multi-octave noise (test single octave first)
- ❌ Recording/frame capture (focus on live performance)
- ❌ Phantom line filtering (CPU-side concern, not GPU)
