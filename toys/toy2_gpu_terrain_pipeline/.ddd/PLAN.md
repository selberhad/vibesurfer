# Toy Model 2: GPU Terrain Pipeline - Implementation Plan

## Overview

**Goal:** Build a GPU compute shader pipeline that generates 1024×1024 terrain at ≥60 FPS, validates audio-reactive parameters, and tests toroidal wrapping on GPU.

**Scope:** Two-phase implementation:
1. Phase 1: Static/animated terrain with audio reactivity (core performance validation)
2. Phase 2: Add toroidal wrapping (integration de-risking)

**Priorities:**
1. Performance first - hit 60 FPS at production scale
2. Correct compute→render integration
3. Visual validation (terrain looks right)
4. Wrapping logic correctness

**Methodology:**
- TDD: Write visual/behavioral tests, then implement
- Test via: Visual inspection, FPS metrics, determinism checks
- No unit tests (GPU code not easily unit testable)
- Integration tests: Run binary, observe output, check metrics

---

## Step 1: Project Scaffolding

### Goal
Set up minimal Rust project with wgpu, window, and FPS display.

### Step 1.a: Write Tests
- **Manual test**: Run binary, see black window with FPS counter
- **Validation**: Window opens, doesn't crash, shows "FPS: 0" initially

### Step 1.b: Implement

**Tasks:**
1. Create `Cargo.toml` with dependencies:
   - `wgpu`, `winit`, `pollster`, `bytemuck`, `env_logger`
2. Create `src/main.rs` with minimal event loop
3. Initialize wgpu (device, queue, surface, swapchain)
4. Render loop: clear to black, present
5. FPS tracking: `Instant::now()`, rolling average over 60 frames
6. Overlay FPS text (simple debug text, use `println!` initially)

**Code pattern:**
```rust
struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    fps_tracker: FpsTracker,
}

impl App {
    fn new(window: &Window) -> Self { /* init wgpu */ }
    fn render(&mut self) {
        // Clear to black, present, update FPS
    }
}

fn main() {
    // Event loop, call app.render() each frame
}
```

### Success Criteria
- [ ] Binary compiles and runs
- [ ] Window opens (1280×720)
- [ ] FPS counter visible (console or overlay)
- [ ] Clean shutdown on close

**Commit:** `feat(toy2): Step 1 - project scaffolding with FPS display`

---

## Step 2: Compute Shader (Static Terrain)

### Goal
Generate 512×512 terrain on GPU (smaller grid for fast iteration), render as points.

### Step 2.a: Write Tests
- **Visual test**: Run binary, see white dots forming terrain heightfield
- **Validation**:
  - Dots form hills (not flat plane)
  - Deterministic (same seed = same pattern)
  - FPS > 100 (512×512 should be fast)

### Step 2.b: Implement

**Tasks:**
1. Copy noise shader from toy1: `src/noise.wgsl` (Stefan Gustavson simplex)
2. Create `src/terrain_compute.wgsl`:
   - Input: `@binding(0) var<storage, read_write> vertices: array<Vertex>`
   - Input: `@binding(1) var<uniform> params: TerrainParams`
   - Workgroup size: 256
   - Logic: Calculate grid position, sample noise (base layer only), write to vertex.y
3. Define structs in `src/main.rs`:
   - `Vertex { position: [f32; 3], uv: [f32; 2] }`
   - `TerrainParams { base_amplitude, base_frequency, grid_size, grid_spacing, ... }`
4. Create compute pipeline:
   - Load shader, create bind group layout (storage buffer + uniform)
   - Create pipeline
5. Allocate buffers:
   - Vertex buffer: `STORAGE | VERTEX`, size = 512×512 vertices
   - Uniform buffer: `UNIFORM | COPY_DST`, size = TerrainParams
6. Dispatch compute:
   - Update uniform buffer with params
   - Create command encoder, compute pass
   - Dispatch `(512*512 + 255) / 256` workgroups
   - Submit
7. Render pass:
   - Simple vertex shader (MVP transform, orthographic camera)
   - Fragment shader (white color)
   - Draw points (not triangles yet)

**Code pattern (shader):**
```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&vertices)) { return; }

    let grid_size = params.grid_size;
    let x = idx % grid_size;
    let z = idx / grid_size;

    let world_x = f32(x) * params.grid_spacing;
    let world_z = f32(z) * params.grid_spacing;

    // Sample noise (use 0.1x scaling from toy1 learnings)
    let height = sample_noise(world_x * 0.1, world_z * 0.1, 0.0) * params.base_amplitude;

    vertices[idx].position = vec3<f32>(world_x, height, world_z);
    vertices[idx].uv = vec2<f32>(f32(x) / f32(grid_size), f32(z) / f32(grid_size));
}
```

### Success Criteria
- [ ] Compute shader compiles without errors
- [ ] Terrain visible as point cloud (white dots)
- [ ] Hills visible (not flat)
- [ ] FPS > 100 at 512×512
- [ ] Same seed produces same terrain (run twice, compare visually)

**Commit:** `feat(toy2): Step 2 - compute shader generates static terrain`

---

## Step 3: Wireframe Rendering

### Goal
Render terrain as wireframe triangles instead of points.

### Step 3.a: Write Tests
- **Visual test**: See wireframe mesh (neon lines, not dots)
- **Validation**:
  - Triangles connect correctly (no random lines)
  - Terrain shape preserved
  - FPS still >100

### Step 3.b: Implement

**Tasks:**
1. Generate index buffer (CPU, once at init):
   - For each grid cell (x, z), create 2 triangles: (i, i+1, i+grid_size), (i+1, i+grid_size+1, i+grid_size)
   - Store as `Vec<u32>`, upload to GPU
2. Update render pipeline:
   - Topology: `TriangleList`
   - Polygon mode: `Line` (wireframe)
   - Line width: 1.0 (platform-dependent, may not work on all GPUs)
3. Render pass:
   - Bind vertex + index buffers
   - `draw_indexed(index_count, 1, 0, 0, 0)`

**Code pattern:**
```rust
fn generate_indices(grid_size: usize) -> Vec<u32> {
    let mut indices = Vec::new();
    for z in 0..grid_size-1 {
        for x in 0..grid_size-1 {
            let i = (z * grid_size + x) as u32;
            // Triangle 1
            indices.extend_from_slice(&[i, i+1, i+grid_size as u32]);
            // Triangle 2
            indices.extend_from_slice(&[i+1, i+grid_size as u32+1, i+grid_size as u32]);
        }
    }
    indices
}
```

### Success Criteria
- [ ] Wireframe mesh visible (triangles, not points)
- [ ] No gaps or missing triangles
- [ ] Terrain shape correct (hills visible)
- [ ] FPS >100 maintained

**Commit:** `feat(toy2): Step 3 - wireframe rendering with indexed triangles`

---

## Step 4: Scale to 1024×1024 (Performance Test)

### Goal
Validate performance at production scale (1,048,576 vertices).

### Step 4.a: Write Tests
- **Performance test**: Run at 1024×1024, measure FPS for 10 seconds
- **Validation**:
  - FPS ≥ 60 consistently (critical threshold)
  - No stuttering or frame drops
  - GPU memory usage acceptable

### Step 4.b: Implement

**Tasks:**
1. Change `grid_size` constant from 512 → 1024
2. Recompile and run
3. Add FPS statistics logging:
   - Min/Max/Avg FPS over 10 seconds
   - Frame time percentiles (p50, p95, p99)
4. If FPS < 60:
   - Profile with Instruments.app (macOS)
   - Check GPU utilization, memory bandwidth
   - Tune workgroup size (try 128, 256, 512)

**Code pattern:**
```rust
struct FpsTracker {
    frame_times: VecDeque<Duration>,
    start: Instant,
}

impl FpsTracker {
    fn record_frame(&mut self, duration: Duration) {
        self.frame_times.push_back(duration);
        if self.frame_times.len() > 600 { // 10 seconds @ 60fps
            self.frame_times.pop_front();
        }
    }

    fn stats(&self) -> (f32, f32, f32) { // min, avg, max FPS
        // Calculate from frame_times
    }
}
```

### Success Criteria
- [ ] 1024×1024 grid renders correctly
- [ ] FPS ≥ 60 sustained for 10 seconds
- [ ] Avg frame time ≤ 16.67ms
- [ ] No GPU errors or validation warnings

**Commit:** `feat(toy2): Step 4 - validate 60+ FPS at 1024×1024 scale`

---

## Step 5: Audio-Reactive Parameters

### Goal
Modulate terrain amplitude and frequency per-frame using simulated audio bands.

### Step 5.a: Write Tests
- **Visual test**: Terrain animates (hills grow/shrink, choppiness varies)
- **Validation**:
  - Amplitude changes visible (bass modulation)
  - Frequency changes visible (mid modulation)
  - FPS ≥ 60 maintained with parameter updates

### Step 5.b: Implement

**Tasks:**
1. Add detail layer to compute shader:
   - Sample noise twice: base (time=0) + detail (time=animated)
   - Combine: `height = base + detail * params.detail_amplitude`
2. Update `TerrainParams`:
   - Add `detail_amplitude`, `detail_frequency`, `time`
3. Simulate audio bands in main loop:
   ```rust
   let audio_bands = AudioBands {
       low: 5.0 + 5.0 * (time * 0.5).sin(),
       mid: 3.0 + 2.0 * (time * 1.0).sin(),
       high: 2.0 + 1.0 * (time * 2.0).sin(),
   };
   ```
4. Update uniform buffer each frame:
   - `detail_amplitude = 2.0 + audio_bands.low * 3.0`
   - `detail_frequency = 0.1 + audio_bands.mid * 0.15`
   - `time = elapsed_seconds`
5. Re-dispatch compute shader each frame (vertices regenerated)

**Code pattern (shader):**
```wgsl
let base_height = sample_noise(world_x * 0.1 * params.base_frequency, world_z * 0.1 * params.base_frequency, 0.0)
                  * params.base_amplitude;

let detail_height = sample_noise(world_x * 0.1 * params.detail_frequency, world_z * 0.1 * params.detail_frequency, params.time)
                    * params.detail_amplitude;

let height = base_height + detail_height;
```

### Success Criteria
- [ ] Terrain animates smoothly (no stuttering)
- [ ] Amplitude scales with `audio_bands.low` (visual confirmation)
- [ ] Frequency scales with `audio_bands.mid` (visual confirmation)
- [ ] FPS ≥ 60 with per-frame updates
- [ ] Frame time breakdown: compute < 2ms, render < 14ms

**Commit:** `feat(toy2): Step 5 - audio-reactive parameter modulation`

---

## Phase 1 Complete: Performance Validated ✓

**Exit Criteria Met:**
- [x] GPU compute shader working
- [x] 1024×1024 terrain at ≥60 FPS
- [x] Audio-reactive parameters functional

**Decision Point:** If Phase 1 fails to hit 60 FPS, **stop here** and document findings in LEARNINGS.md. No point testing wrapping if core performance fails.

---

## Step 6: Camera Position Tracking (Phase 2 Start)

### Goal
Add camera position to shader for world-space coordinate calculations (prep for wrapping).

### Step 6.a: Write Tests
- **Visual test**: Terrain appears to "scroll" as camera position changes
- **Validation**:
  - Same world position = same height (coordinate system correct)
  - No visual change if camera stationary (backward compatibility)

### Step 6.b: Implement

**Tasks:**
1. Add `camera_pos: vec3<f32>` to `TerrainParams`
2. Update compute shader:
   - Calculate world position: `world_pos = grid_pos + camera_offset`
   - Sample noise at world position (not grid position)
3. Animate camera position in main loop:
   ```rust
   camera_pos.z += delta_time * speed; // Move forward
   ```
4. Update uniform buffer with camera_pos each frame

**Code pattern (shader):**
```wgsl
let grid_x = f32(x) * params.grid_spacing;
let grid_z = f32(z) * params.grid_spacing;

let world_x = grid_x - params.camera_pos.x;
let world_z = grid_z - params.camera_pos.z;

let height = sample_noise(world_x * 0.1, world_z * 0.1, params.time) * params.amplitude;
```

### Success Criteria
- [ ] Terrain "scrolls" as camera moves forward
- [ ] Same world XZ = same height (test by comparing frames)
- [ ] FPS ≥ 60 maintained
- [ ] Visual continuity (no jumps or glitches)

**Commit:** `feat(toy2): Step 6 - camera position tracking for world-space coords`

---

## Step 7: Toroidal Wrapping

### Goal
Wrap vertices when they exit camera view, creating infinite scrolling terrain.

### Step 7.a: Write Tests
- **Visual test**: Terrain scrolls infinitely with no seams
- **Validation**:
  - No gaps at wrap boundary
  - Noise coherent across wrap (same world pos = same height)
  - FPS ≥ 60 maintained
  - Camera can move for 60+ seconds without issues

### Step 7.b: Implement

**Tasks:**
1. Define wrap boundary in compute shader:
   - Grid extent: `grid_size * grid_spacing`
   - Wrap threshold: `camera_pos.z - grid_extent / 2`
2. Apply wrapping logic per vertex:
   ```wgsl
   if (world_z < wrap_threshold) {
       world_z += grid_extent;
   }
   ```
3. Sample noise at wrapped world coordinates
4. Test visually: run for 60 seconds, look for seams

**Code pattern (shader):**
```wgsl
let grid_extent = f32(params.grid_size) * params.grid_spacing;
let wrap_threshold = params.camera_pos.z - grid_extent * 0.5;

var world_z = grid_z - params.camera_pos.z;
if (world_z < wrap_threshold) {
    world_z += grid_extent;
}

// Now sample noise at wrapped coordinates
let height = sample_noise(world_x * 0.1, world_z * 0.1, params.time) * params.amplitude;
```

### Success Criteria
- [ ] Terrain scrolls infinitely (no end)
- [ ] No visible seams at wrap boundary
- [ ] Noise coherence maintained (world-space sampling correct)
- [ ] FPS ≥ 60 for 60+ seconds of continuous scrolling
- [ ] Camera can move >10km without issues

**Commit:** `feat(toy2): Step 7 - toroidal wrapping for infinite terrain`

---

## Phase 2 Complete: Integration De-Risked ✓

**Exit Criteria Met:**
- [x] Toroidal wrapping works on GPU
- [x] No seams at boundaries
- [x] Performance maintained (≥60 FPS)
- [x] World-space coordinates validated

---

## Step 8: Benchmarking & Profiling

### Goal
Measure and document performance characteristics for different grid sizes.

### Step 8.a: Write Tests
- **Benchmark**: Run at 256, 512, 1024, 2048 (if GPU supports)
- **Metrics**: FPS, frame time breakdown (compute vs render), GPU memory

### Step 8.b: Implement

**Tasks:**
1. Add GPU timestamp queries (if supported):
   - Query compute pass duration
   - Query render pass duration
2. Run benchmark suite:
   - Each grid size for 10 seconds
   - Log min/avg/max FPS, frame times
3. Document in LEARNINGS.md:
   - Performance scaling curve
   - GPU utilization
   - Bottleneck analysis

**Code pattern:**
```rust
let compute_time = query_timestamp(compute_pass);
let render_time = query_timestamp(render_pass);
println!("Compute: {:.2}ms, Render: {:.2}ms", compute_time, render_time);
```

### Success Criteria
- [ ] Benchmarks run at 256, 512, 1024 grid sizes
- [ ] Frame time breakdown measured (compute vs render)
- [ ] Performance data documented in LEARNINGS.md
- [ ] Bottleneck identified (if any)

**Commit:** `feat(toy2): Step 8 - benchmark performance across grid sizes`

---

## Step 9: Visual Polish (Optional)

### Goal
Make terrain more readable (camera angle, neon glow, skybox).

### Step 9.a: Write Tests
- **Visual test**: Terrain looks "vibesurfer-like" (neon wireframe aesthetic)

### Step 9.b: Implement

**Tasks:**
1. Adjust camera:
   - Angle: look down at ~30° (not top-down)
   - Height: 50-100m above terrain
2. Add neon glow to wireframe:
   - Fragment shader: emit color with alpha
   - Enable alpha blending
3. Add simple skybox (gradient, like main game)

### Success Criteria
- [ ] Camera angle shows terrain depth
- [ ] Wireframe has neon aesthetic (cyan/magenta glow)
- [ ] Skybox visible (not black void)

**Commit:** `feat(toy2): Step 9 - visual polish (camera, glow, skybox)`

---

## Final Validation

### All Tests Pass
- [ ] Phase 1: Static/animated terrain at 60+ FPS
- [ ] Phase 1: Audio-reactive parameters working
- [ ] Phase 2: Toroidal wrapping with no seams
- [ ] Phase 2: FPS maintained with wrapping
- [ ] Benchmarks documented

### Ready for Integration
- [ ] LEARNINGS.md written with performance data
- [ ] README.md written with usage instructions
- [ ] Code ready to reference in main codebase integration

---

## Risk Mitigation

**If FPS < 60 at 1024×1024:**
- Profile with Instruments.app (GPU trace)
- Check compute shader occupancy (workgroup size tuning)
- Validate memory bandwidth (buffer sizes, stride alignment)
- Consider LOD approach (variable grid density)
- **Document findings in LEARNINGS.md**

**If wrapping has seams:**
- Verify world-space coordinate math (add debug visualization)
- Check noise sampling (same world pos should = same height)
- Test wrap boundary explicitly (add visual marker at wrap line)
- **Document edge cases in LEARNINGS.md**

**If GPU errors occur:**
- Check buffer bounds (use `arrayLength()` in shader)
- Validate bind group layouts match shader bindings
- Add wgpu validation layer logging
- **Capture error logs in LEARNINGS.md**

---

## Success Definition

**Minimum Viable Success:**
- 1024×1024 terrain at ≥60 FPS (Phase 1)
- Toroidal wrapping works with no seams (Phase 2)
- Performance data documented
- Integration path clear

**Stretch Goals:**
- 2048×2048 at 60+ FPS
- Frame time breakdown optimized (compute < 2ms)
- Visual polish (neon glow, camera polish)
- Deterministic replay (same seed + input = same frames)
