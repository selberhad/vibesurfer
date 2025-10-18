# PHASE 0 — GPU Compute Shader Research Complete

**Date**: 2025-10-17
**Phase**: Research mode - Priorities 0-2 study complete
**Status**: ✅ Ready for Discovery

---

## Summary

Systematic study of GPU compute shader programming (WGSL + wgpu) for ocean mesh deformation. Covered foundational concepts → practical patterns → Rust integration.

**Deliverables**:
- 3 learning documents (fundamentals, patterns, integration)
- 1 study plan (priorities + open questions)
- Cached external sources (`.webcache/wgsl/`)

**Time**: ~1 session (as planned)

---

## Questions Answered

### From STUDY_PLAN.md Pre-Study Questions

**Execution Model:**
1. ✅ **Why exactly 256 threads per workgroup?**
   - Answer: GPU hardware runs threads in SIMD lockstep (16-64 simultaneous)
   - 256 = multiple SIMD groups, good occupancy without hitting 256 limit
   - 64 recommended default (WebGPU), 256 conservative for "probably near-optimal"
   - M1 GPU: 128-thread SIMD → 256 = 2 SIMD groups

2. ✅ **What happens if `vertex_count % 256 != 0`?**
   - Answer: Overshoot threads (round up dispatch)
   - Must bounds-check in shader: `if (idx >= arrayLength(&vertices)) { return; }`
   - Last workgroup partially idle (no harm, just wasted thread slots)

3. ⚠️ **Can we dispatch multiple workgroups in 2D/3D grid?**
   - Answer: Yes, `dispatch_workgroups(x, y, z)` creates 3D grid
   - Useful for 2D/3D spatial data (images, grids)
   - Ocean mesh: 1D dispatch (linear vertex array, no spatial locality benefit)
   - *Open question*: Would 2D dispatch help cache locality for mesh?

**Memory:**
4. ✅ **What's the actual uniform buffer size limit on M1?**
   - Answer: 64KB typical (WebGPU default `maxUniformBufferBindingSize`)
   - Our `ComputeParams`: 32 bytes (well under limit)

5. ✅ **Does alignment matter for performance or just correctness?**
   - Answer: Both
   - Correctness: GPU reads wrong offsets if misaligned
   - Performance: Cache line boundaries (128 bytes typical) - aligned access faster
   - Uniform buffers: Must align to 16 bytes (structs)

6. ⚠️ **Can we use shared memory for noise computation?**
   - Answer: Probably not beneficial
   - Shared memory useful for neighbor access (stencil ops, reduction)
   - Noise: Embarrassingly parallel (each thread independent)
   - *Open question*: Could cache gradient table in shared memory for Perlin?

**Integration:**
7. ⏳ **How do we profile GPU time separately from CPU time?**
   - Answer: Timestamp queries (requires `TIMESTAMP_QUERY` feature)
   - Place timestamps at compute pass begin/end
   - *Deferred*: Implement when profiling needed

8. ✅ **What's the synchronization overhead between compute and render passes?**
   - Answer: Implicit barrier between passes in same encoder (zero overhead)
   - Separate submits: GPU work queue serialization (minimal CPU cost)
   - No explicit sync needed (GPU handles coherency)

9. ⚠️ **Should we double-buffer vertex data?**
   - Answer: Not necessary
   - Compute writes, render reads - no read-write hazard (different passes)
   - Double buffering useful for async compute (overlap compute with render)
   - *Open question*: Would async compute boost framerate?

**Correctness:**
10. ⏳ **How to validate compute shader output matches CPU reference?**
   - Answer: Copy storage buffer to `MAP_READ` buffer, compare on CPU
   - Pattern: `copy_buffer_to_buffer()` → `map_async()` → read typed slice
   - *Deferred*: Implement validation test during Discovery

11. ⏳ **What debugging tools exist for WGSL?**
   - Answer: Limited
   - Validation layers: `RUST_LOG=wgpu=warn` (syntax errors, binding mismatches)
   - GPU debuggers: RenderDoc (limited WGSL support), Xcode Instruments (Metal backend)
   - No printf debugging (write to debug buffer workaround)
   - *Deferred*: Explore during integration if needed

12. ✅ **How to handle shader compilation errors gracefully?**
   - Answer: Currently panics (can't catch easily)
   - Validation during dev: Enable wgpu logging
   - Production: Pre-validate shaders at build time (test harness)
   - *Open question*: Worth adding error screen vs panic?

---

## New Questions Raised

### Performance (Priority 3 - Defer)
13. Is 256 optimal for M1 GPU, or should we benchmark 64/128/256? (Profiling needed)
14. Does hash-based noise produce visible artifacts at certain frequencies? (Visual inspection)
15. How many noise octaves fit in frame budget? (1 vs 2 vs 3 - profile)
16. Would SoA layout measurably improve bandwidth? (GPU profiler - may be compute-bound, not memory-bound)

### Quality (Priority 4 - Defer)
17. Would Perlin/Simplex noise look noticeably better than hash-based? (Subjective A/B test)
18. Is cubic smoothstep sufficient, or do we need quintic for better normals? (Lighting inspection)

### Architecture (Meta - Consider)
19. Would 2D dispatch (workgroup grid matching mesh grid) improve cache locality? (Measure)
20. Could async compute overlap with render? (Advanced - requires double buffering)

---

## Decisions Made

**Workgroup size**: 256 threads
- Rationale: Conservative choice, likely near-optimal for most GPUs
- M1 GPU: 256 = 2× SIMD width (128), good occupancy
- Can benchmark later if performance insufficient

**Noise algorithm**: Hash-based value noise (not Perlin/Simplex)
- Rationale: Simpler implementation, no gradient table, pure arithmetic
- Trade-off: Slightly blockier than gradient noise (acceptable for detail layer)
- Upgrade path: Implement gradient noise if visual quality insufficient

**Buffer layout**: Array of Structures (AoS)
- Rationale: Simpler integration with render pipeline (single vertex buffer)
- Trade-off: Wastes bandwidth updating only position.y (reads full Vertex)
- Refactor to SoA only if profiling shows memory bandwidth bottleneck

**Hybrid CPU/GPU**: Base terrain (CPU) + detail layer (GPU)
- Rationale: Quality (OpenSimplex) + speed (massive parallelism)
- Base: Infrequent updates (only on wrap), cache-friendly
- Detail: Every frame, embarrassingly parallel

**Synchronization**: Separate submits (compute → render)
- Rationale: Simpler code (dedicated methods), clear separation
- Alternative considered: Single encoder (compute + render)
- Performance: Negligible difference (implicit barrier either way)

---

## Learning Artifacts

### Documents Created

1. **`learnings/gpu_compute_fundamentals.md`** (Priority 0)
   - Thread hierarchy (invocation → workgroup → dispatch)
   - WGSL syntax (entry points, bindings, address spaces)
   - Memory model (hierarchy, alignment, storage vs uniform)
   - Workgroup sizing strategy
   - Synchronization patterns

2. **`learnings/wgsl_compute_patterns.md`** (Priority 1)
   - Compute pattern taxonomy (embarrassingly parallel, reduction, stencil)
   - Noise functions (hash-based, value noise, gradient noise)
   - Octave layering (fractal noise)
   - Buffer access patterns (coalesced access, AoS vs SoA)
   - Hybrid CPU/GPU strategy

3. **`learnings/wgpu_compute_integration.md`** (Priority 2)
   - Buffer creation (usage flags, initialization)
   - Shader module loading
   - Bind group layout + bind group
   - Pipeline layout + compute pipeline
   - Dispatch calculation
   - Synchronization (compute → render, CPU ← GPU)
   - Struct alignment (bytemuck, `#[repr(C)]`)

### External Sources Cached

- `.webcache/wgsl/webgpu_compute_fundamentals.html` (33KB)
- `.webcache/wgsl/tour_of_wgsl.html` (13KB)
- `.webcache/wgsl/noise_patterns.html` (Book of Shaders)

### Code References

- `src/ocean_compute.wgsl` - GPU noise implementation (already exists)
- `src/noise.rs` - CPU OpenSimplex wrapper
- `src/rendering.rs` - Existing render pipeline (pattern for compute pipeline)

---

## Coverage Assessment

### STUDY_PLAN.md Checkboxes

**Priority 0** (Foundational): ✅ Complete
- [x] WGSL basics (syntax, bindings, address spaces)
- [x] GPU execution model (threads, workgroups, dispatch)
- [x] Memory model (hierarchy, alignment, buffer types)

**Priority 1** (Core): ✅ Complete
- [x] Common compute patterns (embarrassingly parallel, reduction, stencil)
- [x] Noise functions on GPU (hash-based, value noise)
- [x] Buffer access patterns (coalesced access, AoS vs SoA)

**Priority 2** (Practical): ✅ Complete
- [x] Pipeline creation (shader module, layouts, pipeline)
- [x] Buffer management (usage flags, initialization, updates)
- [x] Compute pass execution (dispatch calculation, encoding)
- [x] Synchronization (implicit barriers, CPU-GPU transfer)

**Priority 3** (Advanced): ⏸️ Deferred
- [ ] Workgroup sizing benchmarks (profile after integration)
- [ ] Memory access optimization (measure before optimizing)
- [ ] Profiling tools (timestamp queries - implement when needed)

**Priority 4** (Specialized): ⏸️ Deferred
- [ ] FFT on GPU (future feature - audio-driven synthesis)
- [ ] Texture sampling in compute (flowfield implementation)
- [ ] Indirect dispatch (GPU-driven LOD)

---

## Readiness Assessment

### Success Criteria (from STUDY_PLAN.md)

✅ **Priorities 0-2 complete and documented**
- 3 learning docs created
- All foundational, core, and practical topics covered

✅ **Can explain shader execution model, memory hierarchy, synchronization**
- Thread hierarchy: invocation → workgroup → dispatch
- Memory: registers → shared → L1/L2 → VRAM
- Sync: Implicit barriers between passes, bounds-check for overshot threads

✅ **Workgroup sizing strategy validated (256 threads - why?)**
- GPU SIMD width (16-64 lockstep threads)
- 256 = multiple SIMD groups, near-optimal for most hardware
- M1 specific: 128-thread SIMD → 256 = 2 groups

✅ **Open questions catalogued (theory vs practice gaps)**
- 20 questions documented (12 answered, 8 deferred/open)
- Clear distinction: answered by research vs needs Discovery validation

✅ **Ready to integrate compute pipeline confidently**
- HANDOFF.md provides step-by-step integration plan (~100 lines)
- Learning docs explain *why* each step exists (not just *what*)
- Can debug if integration fails (understand error modes)

---

## Transition to Discovery

### Next Steps

**Ready to exit Research mode → enter Discovery mode**

**Discovery phase tasks** (from methodology):
1. **Build toy** (progressive complexity):
   - Step 1: Trivial compute (array doubling - verify execution)
   - Step 2: Add hash-based noise (validate GPU noise output)
   - Step 3: Update vertex buffer (match ocean mesh pattern)
   - Step 4: Integrate into main (copy working code)

2. **Validate assumptions**:
   - GPU noise quality (compare to CPU visually)
   - Performance gain (measure FPS before/after)
   - Correctness (spot-check vertex heights)

3. **Document learnings**:
   - What worked as expected?
   - What surprised us? (edge cases, performance, visual quality)
   - What to optimize? (profiling insights)

**Alternative**: Skip toy, integrate directly (HANDOFF.md is detailed enough)
- Pros: Faster to working feature
- Cons: Riskier (less validation of understanding)
- Recommendation: Build toy first (one file, < 200 lines, 30-60 min)

---

## Meta-Learnings (About Research Process)

### What Worked Well

**Prioritized study plan**: Clear progression (foundations → patterns → integration)
- Avoided random walk through docs
- Each priority built on previous
- Knew when to stop (time-boxed)

**Synthesis over transcription**: Learning docs extract patterns/constraints/gotchas
- Not wiki copy-paste
- Domain language (what it means, not how it works)
- Cross-referenced sources (attributions, further reading)

**Cached sources**: `.webcache/` for offline, stable references
- Faster than repeated HTTP fetches
- Version stability (wiki pages change)
- Added to `.gitignore` (don't commit cache)

### What to Improve

**WebFetch attempted first**: Should have gone straight to curl + lynx
- CLAUDE.md mentions webcache protocol
- Lesson: RTFM before tool selection

**Incomplete source coverage**: Some URLs not cached (wgpu docs, Learn wgpu)
- Relied on existing code (`src/rendering.rs`) for wgpu patterns
- Could cache more examples for future reference
- Trade-off: Time-boxing prevented over-caching

**No validation tests**: Research phase pure study (no code execution)
- Can't verify understanding until Discovery
- Could have written simple tests (compile checks, size assertions)
- Acceptable: Discovery phase is for validation

---

## Remember

From LEXICON.md:
> "Artifacts are disposable, clarity is durable. Code can be rewritten, insights cannot."

**This research phase embodied that principle:**
- External sources disposable (cached for reference, not copied verbatim)
- Learning docs durable (synthesized understanding, optimized for AI-human collaboration)
- Open questions explicit (theory vs practice gaps)

**Study revealed**:
- What we *understand* (execution model, memory hierarchy, wgpu API)
- What we need to *validate* (noise quality, performance gains, correctness)

**Discovery mode tests understanding against reality.**

---

**Assessment**: ✅ Research phase complete. Ready for Discovery.

**Recommendation**: Build progressive toy (4 steps) before main integration.

**Estimated Discovery time**: 1-2 sessions (toy + integration + learnings)
