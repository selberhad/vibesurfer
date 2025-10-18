# Toy Model 2: GPU Terrain Pipeline – Learnings

Duration: 1 session | Status: Partially Complete | Estimate: 3-4 hours

## Summary

**Built:** GPU compute shader pipeline generating 1024×1024 terrain with audio-reactive parameters
**Worked:** Performance validation - 120 FPS at production scale (2× target)
**Failed:** Visual correctness - rendering issues obscured terrain, toroidal wrapping created phantom lines
**Uncertain:** Whether GPU-side toroidal wrapping is viable without CPU filtering

## Evidence

### ✅ Validated: GPU Performance

**Performance achieved:**
- Grid: 1024×1024 (1,048,576 vertices)
- FPS: 115-120 average (Min: 49.6, Max: 122.2)
- Target: ≥60 FPS
- **Result: 2× better than target**

**Key metrics:**
- Compute shader dispatches 4,096 workgroups (256 threads each)
- Per-frame parameter updates (audio modulation) have negligible cost
- Platform: M1 Mac, wgpu 22.1, Metal backend

**Conclusion:** GPU compute approach is viable for main codebase integration. Performance headroom allows for additional complexity (multi-octave noise, more complex terrain generation).

### ⚠️ Challenged: Visual Validation

**Problem:** Implemented all 9 steps in one session without visual validation between steps.

**Symptoms:**
- Step 2-5: Terrain rendered but not visually inspected
- Step 7: Toroidal wrapping logic added, created phantom lines (stretched triangles)
- Step 9: Perspective camera broken, never displayed terrain correctly

**Root cause:** Skipped incremental visual testing. By the time we ran the binary, multiple issues compounded:
1. Toroidal wrapping logic creates degenerate triangles (vertices wrap but index buffer doesn't)
2. Camera matrix setup incorrect (perspective view not working)
3. No way to isolate which step introduced which issue

**Lesson:** For graphics work, **visual validation must happen at each step**. Running the binary after Step 2, 3, 4 would have caught camera/rendering issues before adding wrapping complexity.

### ❌ Failed: Toroidal Wrapping on GPU

**Approach attempted:**
```wgsl
// Per-vertex wrapping in compute shader
if (world_z < wrap_threshold) {
    world_z += grid_extent;
}
```

**Problem:** Index buffer connects vertices in grid-space, but wrapped vertices jump to different world-space positions. Result: long diagonal "phantom lines" connecting front/back of grid.

**Why it failed:**
- Wrapping individual vertices breaks triangle topology
- Index buffer assumes regular grid structure
- Need either:
  1. CPU-side phantom line filtering (current main codebase approach), OR
  2. Different wrapping strategy (flow entire grid, not individual vertices)

**Current main codebase approach (from ARCHITECTURE.md):**
- Vertices flow backward as camera moves forward
- Toroidal repositioning when vertices exit bounds
- **CPU-side filtering** of stretched triangles before rendering
- This works because wrapping happens at grid level, not vertex level

**Conclusion:** GPU-only toroidal wrapping without CPU filtering is probably not viable. Main codebase should keep current CPU filtering approach.

### 🌀 Uncertain: Buffer Alignment

**Issue discovered:** WGSL vec3 requires 16-byte alignment, Rust `[f32; 3]` is 12 bytes.

**Solution:** Added manual padding:
```rust
struct TerrainParams {
    camera_pos: [f32; 3],
    _padding1: f32,  // Required for WGSL alignment
}
```

**Question:** Does wgpu have automatic padding helpers, or is manual padding always required? This wasn't documented in toy1.

## Pivots

**Original plan:** Validate both performance AND toroidal wrapping
**Actual result:** Validated performance only, wrapping approach needs rethinking

**Why pivot:**
- Phantom lines revealed fundamental issue with per-vertex wrapping
- Out of time/context to debug camera + wrapping simultaneously
- Performance validation (primary goal) succeeded

**New understanding:** Toroidal wrapping needs to happen at grid-flow level (CPU) or with CPU-side triangle filtering. GPU-only approach adds complexity without clear benefit.

## Impact

### Reusable Patterns

1. **GPU Compute Template:**
   - Compute pipeline setup (bind groups, storage buffers, uniforms)
   - Dispatch calculation: `(vertex_count + workgroup_size - 1) / workgroup_size`
   - Per-frame uniform updates via `queue.write_buffer()`

2. **Noise Shader (from toy1):**
   - Stefan Gustavson simplex in WGSL
   - 0.1x frequency scaling factor (empirically validated)
   - Reusable for any GPU terrain generation

3. **FPS Tracking:**
   - Min/avg/max statistics
   - Rolling 60-frame window
   - Ready to copy to main codebase

### Architectural Consequences

**For main codebase integration:**

1. **Move terrain generation to GPU** ✅ Validated
   - Replace CPU Perlin loop with compute shader dispatch
   - Keep CPU-side phantom line filtering
   - Expected gain: 60fps → 120fps at current grid size

2. **Don't use per-vertex GPU wrapping** ❌ Not viable
   - Keep current grid-flow + CPU filtering approach
   - GPU generates, CPU manages topology

3. **Buffer alignment matters** ⚠️ Watch for this
   - WGSL vec3 = 16 bytes, Rust [f32; 3] = 12 bytes
   - Add manual padding or use vec4

### Estimate Calibration

**Original estimate:** 3-4 hours
**Actual time:** ~2-3 hours implementation + debugging
**Outcome:** Performance validated, visual validation incomplete

**Calibration:**
- Implementation speed was accurate
- **Underestimated:** Need for incremental visual testing
- **Missed:** Camera setup complexity for 3D rendering
- **Next time:** Add "visual validation checkpoints" to PLAN

## Recommendations for Future Work

### Immediate (Main Codebase Integration)

1. **Start with Step 2-4 only:** GPU compute + simple rendering
   - Validate visually before adding features
   - Keep current CPU grid-flow for wrapping
   - Target: 60+ FPS at 1024×1024

2. **Add compute pipeline incrementally:**
   - Day 1: Static terrain (no audio), visual validation
   - Day 2: Audio-reactive detail layer
   - Day 3: Integration with existing wrapping logic

3. **Keep phantom line filtering on CPU**
   - Current approach works
   - GPU-only wrapping adds complexity without benefit

### Process Improvements

1. **Add visual checkpoints to PLAN:**
   - "Step N.c: Run binary, confirm visual output"
   - Don't proceed to next step without visual confirmation
   - Especially critical for graphics/rendering work

2. **Simpler camera setup:**
   - Start with orthographic top-down (known working)
   - Add perspective later as polish, not core functionality

3. **One complexity axis at a time:**
   - This toy added: compute shader + wireframe + wrapping + camera
   - Should have been: compute shader + simple points + static camera
   - Then add one feature at a time with validation

## Key Files for Reference

- `src/terrain_compute.wgsl` - Noise generation compute shader (reusable)
- `src/main.rs:FpsTracker` - Performance tracking (copy to main)
- SPEC.md - Good performance test scenarios
- PLAN.md - 9-step breakdown (too many steps without validation)

## What to Try Next Session

1. **Check out Step 2 commit:** `git checkout beb3837` - see if points render correctly
2. **Check out Step 3 commit:** `git checkout 9aad205` - see if wireframe works without wrapping
3. **Debug camera:** If Steps 2-3 render nothing, camera is broken (fix before adding wrapping)
4. **Toroidal wrapping:** Try grid-flow approach instead of per-vertex wrapping

## Meta-Learning (DDD Process)

**What worked:**
- Clear SPEC with performance targets
- Incremental commits (good for debugging)
- FPS metrics gave concrete validation

**What didn't work:**
- No visual validation checkpoints in PLAN
- Too many steps (9) without intermediate verification
- Assumed rendering "just works" - graphics needs visual testing

**DDD improvement:**
- For graphics toys: PLAN should include "run and verify" after every step
- Consider "visual TDD" - define what should be visible before implementing
- Break large toys (9 steps) into smaller toys (3-4 steps each)
