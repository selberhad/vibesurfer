# GPU Terrain Pipeline Refactor Plan

**Goal**: Move terrain generation from CPU to GPU compute shaders, validated by toy2 performance results (115 FPS at 1024×1024 grid, 2× target).

**Status**: Phase 1 complete, blocked on camera refactor
**Estimated effort**: 3-4 sessions (incremental, reversible)

**Progress**:
- ✅ Phase 1 complete (10× FPS improvement: 12-15 → 120 FPS)
- ⏸️ Phase 2 blocked (requires camera stability fix)
- ⏸️ Phase 3 blocked (depends on Phase 2)

**Next**: See CAMERA_REFACTOR.md for camera system fixes needed to unblock Phase 2

---

## Context

### Current Architecture (CPU-based)

**Per-frame flow** (`main.rs:169-236`):
1. CPU: `ocean.update()` → calls `grid.update()` in `ocean/mesh.rs:123-196`
2. CPU: For each vertex (1024×1024 = 1,048,576):
   - Flow vertices backward (camera motion)
   - Toroidal wrapping
   - Sample base terrain (Perlin, cached if not wrapped)
   - Sample detail layer (Perlin, animated)
   - Combine: `height = base + detail`
   - Filter stretched triangles (~4ms total)
3. CPU→GPU: Upload vertices via `render_system.update_vertices()` (rendering.rs:337-340)
4. CPU→GPU: Upload filtered indices via `render_system.update_indices()` (rendering.rs:342-346)
5. GPU: Render ocean mesh

**Performance**: ~12ms frame time (60 FPS with 30% headroom)

### Target Architecture (GPU-based)

**Per-frame flow**:
1. CPU: Update uniform buffer (camera pos, audio params, time)
2. GPU Compute: Generate all vertices (base + detail terrain)
3. GPU Render: Draw from compute output buffer
4. GPU→CPU (async): Copy vertex buffer to staging (for physics queries, 1-frame lag)

**Expected performance**: 115 FPS at 1024×1024 (validated by toy2)

---

## Key Design Decisions

### 1. Physics Readback Strategy ✅

**Decision**: 1-frame-latency async readback
- Physics queries terrain from frame N-1 while rendering frame N
- Acceptable for Tribes-style skiing (standard in game engines)
- Cost: ~1ms staging buffer copy (measured in toy2)

**Implementation**:
- Storage buffer (GPU-owned vertices) → Staging buffer (GPU→CPU copy)
- CPU maps staging buffer, reads heights without blocking render thread
- Physics interpolates from vertex grid for smooth terrain queries

### 2. Phantom Line Handling ✅

**Decision**: Accept phantom lines initially, fix later
- Current CPU approach: `filter_stretched_triangles()` culls 2-5% of triangles
- GPU approach: Skip filtering for initial refactor
- Future: Explore elegant GPU solution (geometry shader, compute-based filtering, or different wrapping strategy)

### 3. Base Terrain Caching ✅

**Decision**: Eliminate cache, recompute both layers every frame on GPU
- Current: CPU caches `base_terrain_heights` to avoid recomputation (~optimization)
- GPU: Compute both base + detail every frame (simplex noise is cheap on GPU)
- Justification: Toy2 achieved 115 FPS with full recomputation at 1024×1024

### 4. Noise Implementation ✅

**Decision**: GPU simplex (Stefan Gustavson) is the new source of truth
- Current CPU: `noise` crate's OpenSimplex (`noise.rs:6-27`) - **will be removed**
- GPU: Stefan Gustavson 3D simplex from toy2 (`terrain_compute.wgsl:27-109`)
- Visual appearance will change - this is expected and acceptable
- CPU physics queries will read from GPU-generated terrain (1-frame lag)

---

## Implementation Phases

### Phase 1: Add GPU Compute Pipeline (2 sessions, incremental)

**Goal**: Add GPU compute infrastructure without removing CPU path (dual codepath)

#### Step 1.1: Create compute shader (terrain_compute.wgsl)

**File**: `src/shaders/terrain_compute.wgsl` (new)

**Contents** (from toy2 `terrain_compute.wgsl`):
```wgsl
// Copy Stefan Gustavson simplex noise (lines 27-109)
// Copy TerrainParams struct (lines 11-22) with alignment padding
// Copy Vertex struct (lines 4-9) with alignment padding
// Copy main compute kernel (lines 112-160)
```

**Modifications needed**:
- Remove toroidal wrapping logic (lines 139-142) - accept phantom lines
- Keep base + detail layer separation (lines 144-152)

**Validation**: Shader compiles without errors

#### Step 1.2: Add compute pipeline to RenderSystem

**File**: `src/rendering.rs`

**Changes**:
1. Add compute pipeline field to `RenderSystem`:
   ```rust
   pub struct RenderSystem {
       // ... existing fields
       compute_pipeline: wgpu::ComputePipeline,
       compute_bind_group: wgpu::BindGroup,
       terrain_params_buffer: wgpu::Buffer,
       staging_buffer: wgpu::Buffer, // For physics readback
   }
   ```

2. In `RenderSystem::new()` (after line 318):
   - Create compute shader module
   - Create compute bind group layout (storage buffer + uniform)
   - Create compute pipeline
   - Modify vertex buffer to support STORAGE usage:
     ```rust
     usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC
     ```
   - Create staging buffer for readback

3. Add compute dispatch method:
   ```rust
   pub fn dispatch_terrain_compute(&self, camera_pos: Vec3, detail_amplitude: f32, detail_frequency: f32, time: f32) {
       // Update terrain params uniform
       // Create command encoder
       // Dispatch compute shader (workgroup_size: 256, dispatch count: vertex_count / 256)
       // Submit commands
   }
   ```

**Validation**:
- Build succeeds
- Shader compiles at runtime (no validation errors)
- Can dispatch compute without crashes

#### Step 1.3: Add feature flag for GPU compute

**File**: `Cargo.toml`

**Changes**:
```toml
[features]
default = []
gpu-terrain = []  # Enable GPU compute terrain generation
```

**File**: `src/main.rs`

**Changes** (in `App::render_frame()` around line 200):
```rust
#[cfg(feature = "gpu-terrain")]
{
    // GPU path: dispatch compute shader
    render_system.dispatch_terrain_compute(effective_camera_pos, amplitude, frequency, time_s);
    // Skip CPU ocean.update() - vertices already on GPU
}

#[cfg(not(feature = "gpu-terrain"))]
{
    // CPU path: existing code
    let (amplitude, frequency, line_width) = self.ocean.update(time_s, &audio_bands, effective_camera_pos);
    render_system.update_vertices(&self.ocean.grid.vertices);
    render_system.update_indices(&self.ocean.grid.filtered_indices);
}
```

**Validation**:
- `cargo run` (default features) → CPU path, existing behavior
- `cargo run --features gpu-terrain` → GPU path, should render terrain
- FPS comparison: GPU should be ~2× faster

**Rollback**: If GPU path crashes or produces wrong visuals, disable feature and continue using CPU

---

### Phase 2: Physics Readback Integration (1 session)

**Goal**: Enable physics queries from GPU-generated terrain

#### Step 2.1: Add async staging buffer readback

**File**: `src/rendering.rs`

**Changes**:
1. Add readback state to `RenderSystem`:
   ```rust
   // Per-frame physics data (1-frame lag)
   physics_heights: Arc<Mutex<Vec<f32>>>,  // Shared with physics system
   readback_pending: Arc<AtomicBool>,      // Track async operation
   ```

2. Add readback method:
   ```rust
   pub fn begin_physics_readback(&self) {
       // Copy vertex buffer to staging buffer
       // Map staging buffer async
       // Extract heights (vertex.position.y for each vertex)
       // Update physics_heights when mapping completes
   }
   ```

3. Call `begin_physics_readback()` at end of frame (after render, before present)

**File**: `src/ocean/mesh.rs`

**Changes**:
1. Add physics readback mode to `OceanGrid`:
   ```rust
   pub struct OceanGrid {
       // ... existing fields
       gpu_physics_heights: Option<Arc<Mutex<Vec<f32>>>>,  // Shared with RenderSystem
   }
   ```

2. Modify `query_base_terrain()` (line 97):
   ```rust
   pub fn query_base_terrain(&self, world_x: f32, world_z: f32, physics: &OceanPhysics) -> f32 {
       if let Some(ref gpu_heights) = self.gpu_physics_heights {
           // GPU mode: interpolate from cached heights (1 frame old)
           // Convert world position to grid index
           // Bilinear interpolation from 4 nearest vertices
           // Return interpolated height
       } else {
           // CPU mode: existing direct noise sampling
           // ... existing code (lines 98-106)
       }
   }
   ```

**Validation**:
- Physics queries return reasonable heights
- No crashes from null pointers or race conditions
- Test with fixed camera: ball should collide with visible terrain

**Success criteria**: Ball physics works in GPU mode (even if 1 frame laggy)

---

### Phase 3: Cleanup and Optimization (1 session)

**Goal**: Remove CPU codepath, optimize GPU pipeline

#### Step 3.1: Remove feature flag, make GPU default

**File**: `Cargo.toml` - Remove `gpu-terrain` feature
**File**: `src/main.rs` - Remove `#[cfg]` branches, keep only GPU path
**File**: `src/ocean/mesh.rs` - Remove CPU noise sampling from `update()`, simplify to just wrapping logic

#### Step 3.2: Move wrapping to GPU (optional, if time permits)

**Current**: CPU flows vertices, GPU generates heights
**Target**: GPU handles both flow + height generation

**File**: `src/shaders/terrain_compute.wgsl`

**Changes**:
- Add `camera_delta` to TerrainParams
- Compute vertex flow in shader: `position -= camera_delta`
- Add toroidal wrapping logic (but accept phantom lines)

**Benefit**: Eliminate CPU vertex loop entirely, further performance gain

#### Step 3.3: Performance profiling

**Tool**: Tracy profiler or manual timing
**Measure**:
- Compute shader dispatch time
- Staging buffer readback time
- Total frame time breakdown

**Target**: 100+ FPS at 1024×1024 (toy2 achieved 115 FPS)

---

## Risk Mitigation

### Rollback Strategy

Each phase is independently reversible:
- **Phase 0**: Delete test file
- **Phase 1**: Disable `gpu-terrain` feature, fall back to CPU
- **Phase 2**: Remove readback, keep physics queries on CPU (requires CPU to regenerate terrain)
- **Phase 3**: Keep feature flag permanently if issues arise

### Known Risks

1. **Noise equivalence failure**
   - CPU OpenSimplex vs GPU simplex may produce different patterns
   - Mitigation: Phase 0 catches this early, can switch CPU to match GPU

2. **Physics lag artifacts**
   - 1-frame-old terrain might cause collision glitches
   - Mitigation: Test with slow camera motion first, validate feel

3. **Phantom lines worse than CPU filtering**
   - GPU wrapping without filtering might be unacceptable
   - Mitigation: Phase 1 catches this visually, can add GPU filtering later

4. **Performance regression**
   - GPU might be slower on non-M1 hardware (integrated GPUs)
   - Mitigation: Keep feature flag, profile on target hardware

---

## Success Criteria

### Phase 1 (GPU Pipeline) ✅ COMPLETE
- [x] Shader compiles without validation errors
- [x] Terrain renders correctly in GPU mode
- [x] FPS improves by 50%+ (achieved: 10× improvement, 120 FPS)
- [x] Can toggle between CPU and GPU with feature flag
- ⚠️ Known issue: Terrain degenerates after 20s (camera position accumulation bug)

### Phase 2 (Physics)
- [ ] Ball collision works with GPU terrain (1-frame lag acceptable)
- [ ] No crashes or race conditions in readback
- [ ] Physics queries return heights within 1m of visual terrain

### Phase 3 (Optimization)
- [ ] GPU-only mode stable (no CPU fallback needed)
- [ ] 100+ FPS sustained at 1024×1024 grid
- [ ] Frame time breakdown shows <10ms total

---

## Files to Modify

### New Files
- [ ] `src/shaders/terrain_compute.wgsl` - GPU compute shader

### Modified Files
- [ ] `Cargo.toml` - Add feature flag (temporary)
- [ ] `src/rendering.rs` - Add compute pipeline, staging buffer, readback
- [ ] `src/main.rs` - Add GPU codepath with feature flag
- [ ] `src/ocean/mesh.rs` - Add GPU physics query mode
- [ ] `src/params/ocean.rs` - Add `TerrainParams` struct for GPU uniforms

### Reference Files (no changes)
- `toys/toy2_gpu_terrain_pipeline/src/terrain_compute.wgsl` - Source template
- `toys/toy2_gpu_terrain_pipeline/src/lib.rs` - Helper functions (matrix, indices)

---

## Testing Strategy

### Unit Tests
- [ ] TerrainParams serialization (bytemuck, alignment)
- [ ] Physics interpolation from grid (bilinear)

### Integration Tests
- [ ] GPU pipeline dispatch (no crashes)
- [ ] Staging buffer readback (data integrity)
- [ ] Feature flag toggle (CPU ↔ GPU)

### Visual Tests
- [ ] Terrain renders (any coherent shape is acceptable)
- [ ] Phantom lines acceptable (visual artifact we'll fix later)
- [ ] Audio reactivity preserved (bass → amplitude, mid → frequency)

### Performance Tests
- [ ] FPS benchmarks (CPU vs GPU)
- [ ] Frame time breakdown (compute, render, readback)
- [ ] Scaling test (512×512, 1024×1024, 2048×2048)

---

## Next Steps

1. ✅ **Phase 1**: Implement GPU compute pipeline with feature flag - COMPLETE
2. ⏭️ **Camera Refactor**: Fix terrain degeneration bug (see CAMERA_REFACTOR.md)
3. 🔜 **Phase 2**: Add physics readback (after camera stable)
4. 🔜 **Phase 3**: Remove CPU fallback, make GPU default

**Blockers**: Phase 2 requires stable camera system (CAMERA_REFACTOR.md Phases A, B, C)

**Note**: CPU terrain generation will be removed entirely in Phase 3. GPU is the single source of truth; CPU only reads back heights for physics queries.
