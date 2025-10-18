# Open Questions — GPU Compute Shaders (Vibesurfer)

**Created**: 2025-10-17
**Purpose**: Central tracking from Research phase (GPU compute shader study)
**Status**: 12 answered, 8 open, 20 total

---

## Quick Summary

**Study complete**: GPU compute shader programming (WGSL + wgpu Rust integration)
- Priority 0: Foundations (execution model, memory hierarchy, WGSL syntax)
- Priority 1: Patterns (compute patterns, noise functions, buffer access)
- Priority 2: Integration (wgpu API, pipeline creation, synchronization)

**Open questions**: 8 (deferred to Discovery/profiling)
**Answered during study**: 12
**Primary blockers**: None - ready for Discovery

**Categories**:
1. Performance Optimization (4 open, 0 answered)
2. Visual Quality (2 open, 0 answered)
3. Architecture (2 open, 0 answered)
4. Execution Model (0 open, 3 answered)
5. Memory (0 open, 3 answered)
6. Integration (0 open, 3 answered)
7. Correctness (0 open, 3 answered)

---

## 1. Performance Optimization

**Theory**: `learnings/gpu_compute_fundamentals.md`, `learnings/wgsl_compute_patterns.md`
**Priority**: Defer until after initial integration

### Workgroup Sizing
**Q1.1**: Is 256 optimal for M1 GPU, or should we benchmark 64/128/256?
- Context: WebGPU recommends 64, we chose 256 conservatively
- M1 GPU has 128-thread SIMD width → 256 = 2 SIMD groups
- Could benchmark to find actual sweet spot
- **Answer via**: Profile compute pass with different workgroup sizes (64, 128, 256, 512)
- **Timing**: After integration works, if performance insufficient

### Noise Quality vs Performance
**Q1.2**: Does hash-based noise produce visible artifacts at certain frequencies?
- Context: Using simplified hash-based noise (not Perlin/Simplex)
- Trade-off: Simpler implementation vs potential blockiness
- May have subtle correlations or periodicity at large coordinates
- **Answer via**: Visual inspection during integration, vary detail_frequency parameter
- **Timing**: Immediate (during first run)

### Octave Layering Cost
**Q1.3**: How many noise octaves fit in frame budget (1 vs 2 vs 3)?
- Context: Currently single-octave detail layer
- Each octave = additional noise() call per vertex
- Multi-scale detail (large + medium + fine waves)
- **Answer via**: Profile compute pass with 1/2/3 octaves, measure frame time
- **Timing**: After integration, if surface needs richer detail

### Memory Bandwidth
**Q1.4**: Would Structure-of-Arrays (SoA) layout measurably improve bandwidth?
- Context: Currently Array-of-Structures (AoS) - update position.y but read/write full Vertex
- SoA: Separate position/uv buffers, only touch position data
- Trade-off: Bandwidth efficiency vs code complexity
- **Answer via**: GPU profiler (Metal Performance HUD on macOS), check memory-bound vs compute-bound
- **Timing**: Only if profiling shows memory bandwidth bottleneck

---

## 2. Visual Quality

**Theory**: `learnings/wgsl_compute_patterns.md` (Noise Functions section)
**Priority**: Defer until initial integration complete

### Gradient Noise Upgrade
**Q2.1**: Would Perlin/Simplex noise look noticeably better than hash-based?
- Context: Hash-based value noise may be blockier than gradient noise
- Upgrade path exists (implement gradient interpolation)
- More complex (gradient table or vec2→vec2 hash function)
- **Answer via**: A/B comparison (side-by-side rendering with both algorithms)
- **Timing**: If visual quality insufficient after integration

### Smoothstep Quality
**Q2.2**: Is cubic smoothstep sufficient, or do we need quintic for better normals?
- Context: Cubic smoothstep (f(t) = 3t² - 2t³) has C1 continuity
- Quintic (f(t) = 6t⁵ - 15t⁴ + 10t³) has C2 continuity
- Visible as subtle ridges in lighting (normal vector kinks at cell boundaries)
- **Answer via**: Lighting inspection with directional light, check for discontinuities
- **Timing**: During visual polish phase

---

## 3. Architecture

**Theory**: `learnings/gpu_compute_fundamentals.md`, `learnings/wgpu_compute_integration.md`
**Priority**: Defer (not blocking, potential future optimization)

### 2D Dispatch for Spatial Locality
**Q3.1**: Would 2D dispatch (workgroup grid matching mesh grid) improve cache locality?
- Context: Currently 1D dispatch (linear vertex array)
- 2D dispatch: workgroups aligned with mesh topology
- Benefit: Adjacent threads in workgroup access spatially adjacent vertices (better coalescing)
- **Answer via**: Benchmark 1D vs 2D dispatch, measure performance difference
- **Timing**: Advanced optimization (only if 1D insufficient)

### Async Compute Overlap
**Q3.2**: Could async compute overlap with render pass for better GPU utilization?
- Context: Currently sequential (compute → render in separate submits)
- Async: Compute for frame N+1 while rendering frame N
- Requires double-buffering vertex data
- **Answer via**: Implement async compute, profile frame time and GPU occupancy
- **Timing**: Advanced optimization (requires significant refactor)

---

## 4. Execution Model (ANSWERED)

**Theory**: `learnings/gpu_compute_fundamentals.md`

### ✅ Workgroup Size Rationale
**Q4.1**: Why exactly 256 threads per workgroup?
- ✅ **ANSWERED**: GPU hardware SIMD lockstep + occupancy
  - Source: `learnings/gpu_compute_fundamentals.md` (Workgroup Sizing Strategy section)
  - GPUs run 16-64 threads simultaneously (same instruction)
  - 256 = multiple SIMD groups, near-optimal for most hardware
  - M1 specific: 128-thread SIMD → 256 = 2 groups
  - Alternative: 64 (WebGPU default), could benchmark
- **Next step**: Use 256 for integration, benchmark later if needed

### ✅ Bounds Checking
**Q4.2**: What happens if `vertex_count % 256 != 0`?
- ✅ **ANSWERED**: Overshoot threads, must bounds-check in shader
  - Source: `learnings/gpu_compute_fundamentals.md` (Built-in Thread Indices section)
  - Round up dispatch: `(count + 255) / 256` workgroups
  - Last workgroup partially idle (no harm)
  - Shader: `if (idx >= arrayLength(&vertices)) { return; }`
  - Already implemented: `ocean_compute.wgsl:46`
- **Next step**: Verify bounds check during integration

### ✅ Multi-Dimensional Dispatch
**Q4.3**: Can we dispatch multiple workgroups in 2D/3D grid?
- ✅ **ANSWERED**: Yes, but no clear benefit for ocean mesh
  - Source: `learnings/gpu_compute_fundamentals.md` (Dispatch Calculation section)
  - `dispatch_workgroups(x, y, z)` creates 3D grid of workgroups
  - Useful for 2D/3D spatial data (images, grids)
  - Ocean mesh: 1D dispatch sufficient (linear vertex array)
  - Related: Q3.1 (2D dispatch for cache locality - needs measurement)
- **Next step**: Use 1D dispatch for now

---

## 5. Memory (ANSWERED)

**Theory**: `learnings/gpu_compute_fundamentals.md` (Memory Model section)

### ✅ Uniform Buffer Size Limit
**Q5.1**: What's the actual uniform buffer size limit on M1?
- ✅ **ANSWERED**: 64KB typical (WebGPU default)
  - Source: `learnings/gpu_compute_fundamentals.md` (Storage Buffer vs Uniform Buffer)
  - WebGPU `maxUniformBufferBindingSize`: 64KB
  - Our `ComputeParams`: 32 bytes (well under limit)
  - No concern for ocean mesh params
- **Next step**: Use uniform buffer for ComputeParams (as planned)

### ✅ Alignment Impact
**Q5.2**: Does alignment matter for performance or just correctness?
- ✅ **ANSWERED**: Both correctness and performance
  - Source: `learnings/gpu_compute_fundamentals.md` (Alignment Requirements section)
  - Correctness: GPU reads wrong offsets if misaligned
  - Performance: Cache line boundaries (128 bytes) - aligned access faster
  - Uniform buffers: Must align to 16 bytes (structs)
  - Already padded: `ComputeParams._padding: Vec2` → 32 bytes total
- **Next step**: Verify struct alignment with `assert_eq!(size_of::<ComputeParams>(), 32)`

### ✅ Shared Memory for Noise
**Q5.3**: Can we use shared memory for noise computation?
- ✅ **ANSWERED**: Probably not beneficial for current pattern
  - Source: `learnings/wgsl_compute_patterns.md` (Compute Pattern Taxonomy)
  - Shared memory useful for neighbor access (stencil, reduction)
  - Ocean noise: Embarrassingly parallel (each vertex independent)
  - Potential use: Cache gradient table for Perlin noise (if we upgrade)
  - Related: Q2.1 (gradient noise upgrade)
- **Next step**: Skip shared memory for hash-based noise

---

## 6. Integration (ANSWERED)

**Theory**: `learnings/wgpu_compute_integration.md`

### ✅ GPU Time Profiling
**Q6.1**: How do we profile GPU time separately from CPU time?
- ✅ **ANSWERED**: Timestamp queries (requires feature flag)
  - Source: `learnings/wgpu_compute_integration.md` (Performance Profiling section)
  - Requires `wgpu::Features::TIMESTAMP_QUERY`
  - Place timestamps at compute pass begin/end
  - Copy query results to mappable buffer, read on CPU
  - Alternative: Xcode Instruments (Metal backend profiling)
- **Next step**: Defer timestamp queries until profiling needed

### ✅ Compute-Render Synchronization
**Q6.2**: What's the synchronization overhead between compute and render passes?
- ✅ **ANSWERED**: Minimal (implicit barriers)
  - Source: `learnings/wgpu_compute_integration.md` (Synchronization section)
  - Same encoder: Implicit barrier between passes (zero overhead)
  - Separate submits: GPU work queue serialization (minimal CPU cost)
  - No explicit sync needed (GPU handles coherency)
  - Ocean mesh: Separate submits (compute method → render method)
- **Next step**: Use separate submits (simpler code)

### ✅ Double Buffering Necessity
**Q6.3**: Should we double-buffer vertex data?
- ✅ **ANSWERED**: Not necessary for current design
  - Source: `learnings/wgpu_compute_integration.md` (Synchronization section)
  - Compute writes, render reads - different passes (no hazard)
  - Double buffering useful for async compute (overlap compute/render)
  - Related: Q3.2 (async compute - advanced optimization)
- **Next step**: Single vertex buffer (as planned)

---

## 7. Correctness (ANSWERED)

**Theory**: `learnings/wgpu_compute_integration.md`

### ✅ Output Validation
**Q7.1**: How to validate compute shader output matches CPU reference?
- ✅ **ANSWERED**: Copy to MAP_READ buffer, compare on CPU
  - Source: `learnings/wgpu_compute_integration.md` (CPU ← GPU section)
  - Pattern: `copy_buffer_to_buffer()` → `map_async()` → read typed slice
  - Can compare GPU vs CPU noise output for same seed/position
  - Useful for initial integration validation
- **Next step**: Implement validation test during Discovery (optional)

### ✅ WGSL Debugging Tools
**Q7.2**: What debugging tools exist for WGSL?
- ✅ **ANSWERED**: Limited options
  - Source: `learnings/wgpu_compute_integration.md` (Error Handling section)
  - Validation layers: `RUST_LOG=wgpu=warn` (syntax errors, binding mismatches)
  - GPU debuggers: RenderDoc (limited WGSL), Xcode Instruments (Metal)
  - No printf debugging (workaround: write to debug buffer)
  - Best: Enable wgpu logging during development
- **Next step**: Set `RUST_LOG=wgpu=warn` during integration

### ✅ Shader Compilation Error Handling
**Q7.3**: How to handle shader compilation errors gracefully?
- ✅ **ANSWERED**: Currently panics, validate during dev
  - Source: `learnings/wgpu_compute_integration.md` (Error Handling section)
  - `create_shader_module()` panics on invalid WGSL (can't catch easily)
  - Mitigation: Pre-validate with wgpu logging
  - Production: Could add error screen vs panic (not critical for now)
  - Related: Q7.2 (debugging tools)
- **Next step**: Accept panic behavior, validate shader during dev

---

## Next Steps to Answer These Questions

### Phase 1: Discovery - Progressive Toy (Answers Q1.2, Q7.1-Q7.3)
1. **Toy Step 1**: Trivial compute (array doubling)
   - Verify execution model (dispatch, bounds checking)
   - Validate wgpu pipeline creation workflow
   - Test error handling (intentional WGSL syntax error)
   - Answers: Q7.3 (error handling)

2. **Toy Step 2**: Add hash-based noise
   - Implement GPU noise function
   - Validate output quality (visual inspection)
   - Compare to CPU reference (optional validation test)
   - Answers: Q1.2 (noise artifacts), Q7.1 (validation method)

3. **Toy Step 3**: Update vertex buffer
   - Match ocean mesh pattern (Vertex struct, base+detail)
   - Test buffer access patterns
   - Verify synchronization (compute → render)
   - Answers: Q7.2 (debugging approach)

4. **Toy Step 4**: Integrate into main
   - Copy working code from toy
   - Replace CPU vertex update with GPU dispatch
   - Measure FPS before/after (validate 15-30x gain hypothesis)

### Phase 2: Integration + Profiling (Answers Q1.1, Q1.3, Q1.4)
1. **Baseline measurement**:
   - FPS with CPU update (current: ~15-20 FPS)
   - FPS with GPU compute (expected: 60+ FPS)

2. **Workgroup size profiling** (if needed):
   - Benchmark 64, 128, 256, 512 threads per workgroup
   - Measure frame time for each
   - Answers: Q1.1 (optimal workgroup size for M1)

3. **Octave experiments** (if richer detail needed):
   - Implement 2-3 octave fractal noise
   - Measure performance cost per octave
   - Answers: Q1.3 (octave count budget)

4. **Memory profiling** (if performance insufficient):
   - Use Metal Performance HUD or Instruments
   - Check compute-bound vs memory-bound
   - Answers: Q1.4 (SoA layout benefit)

### Phase 3: Visual Polish (Answers Q2.1, Q2.2)
1. **Lighting inspection**:
   - Add directional light to scene
   - Check for normal discontinuities (ridges at noise cell boundaries)
   - If visible: Upgrade to quintic smoothstep
   - Answers: Q2.2 (smoothstep quality)

2. **Noise quality comparison** (if blockiness visible):
   - Implement gradient noise (Perlin-style)
   - A/B comparison (hash-based vs gradient)
   - Visual preference test
   - Answers: Q2.1 (gradient noise upgrade worth it)

### Phase 4: Advanced Optimization (Answers Q3.1, Q3.2)
1. **Spatial dispatch experiment**:
   - Convert to 2D workgroup dispatch (16×16)
   - Measure cache locality improvement (if any)
   - Answers: Q3.1 (2D dispatch benefit)

2. **Async compute prototype**:
   - Double-buffer vertex data
   - Overlap compute (frame N+1) with render (frame N)
   - Measure GPU occupancy improvement
   - Answers: Q3.2 (async compute value)

---

## Status: ✅ Ready for Discovery

**Research phase complete**:
- All foundational questions answered
- Open questions catalogued with practice paths
- No blockers to starting Discovery

**Primary Discovery path**:
- Build progressive toy (4 steps, ~1 session)
- Integrate into main ocean mesh (~0.5 session)
- Validate performance gain (measure FPS)
- Document learnings (what worked, surprises, optimizations)

**Deferred to future**:
- Performance profiling (Q1.1, Q1.3, Q1.4) - only if needed
- Visual polish (Q2.1, Q2.2) - subjective quality assessment
- Advanced optimization (Q3.1, Q3.2) - nice-to-have

**Theory → Practice transition**: Questions bridge research understanding to experimental validation. Each open question maps to specific toy experiment or measurement.
