# Research Plan - GPU Compute Shaders (WGSL)

**Goal**: Build foundational knowledge for implementing GPU compute shaders in Vibesurfer's ocean mesh deformation system using WGSL (WebGPU Shading Language).

**Context**: We have a working compute shader spike (`ocean_compute.wgsl`) but need deeper understanding to:
- Debug shader compilation issues
- Optimize performance (workgroup sizing, memory access patterns)
- Extend to more complex effects (FFT synthesis, flowfield integration)
- Understand GPU architecture constraints for this platform (M1 MacBook Pro ARM64)

**Success criteria**:
- [ ] Priorities 0-2 complete and documented
- [ ] Learning docs created for key topics
- [ ] Can explain shader execution model, memory hierarchy, synchronization
- [ ] Workgroup sizing strategy validated (256 threads - why?)
- [ ] Open questions catalogued (theory vs practice gaps)
- [ ] Ready to integrate compute pipeline confidently

---

## Priority 0: Foundational (MUST KNOW)

**Target**: 0.5 session

### WGSL Basics
- [ ] Language syntax overview (types, operators, built-ins)
- [ ] Compute shader entry points (`@compute`, `@workgroup_size`)
- [ ] Variable bindings (`@group`, `@binding`, address spaces)
- [ ] Address spaces: `storage`, `uniform`, `private`, `workgroup`
- [ ] Why we can't use `read_write` storage in vertex shaders

### GPU Execution Model
- [ ] Thread → Invocation → Workgroup → Dispatch hierarchy
- [ ] What happens when we call `dispatch_workgroups(n, 1, 1)`
- [ ] Thread indexing: `global_invocation_id`, `local_invocation_id`, `workgroup_id`
- [ ] Why 256 threads per workgroup (hardware constraints)

### Memory Model
- [ ] GPU memory hierarchy: registers → shared → L1/L2 → VRAM
- [ ] Storage buffer vs uniform buffer (size limits, access patterns)
- [ ] Alignment requirements (why we need `_padding` in ComputeParams)
- [ ] Read-only vs read-write storage buffers

**Learning doc**: `learnings/gpu_compute_fundamentals.md`

---

## Priority 1: WGSL Compute Patterns (CORE)

**Target**: 0.5 session

### Common Compute Patterns
- [ ] Embarrassingly parallel (our ocean mesh case)
- [ ] Reduction operations (sum, max, min across threads)
- [ ] Prefix sums (scan algorithms)
- [ ] Shared memory usage within workgroup
- [ ] Barrier synchronization (`workgroupBarrier()`)

### Noise Functions on GPU
- [ ] Hash functions (pseudo-random from deterministic input)
- [ ] Perlin noise implementation (gradient noise)
- [ ] Simplex noise vs Perlin (OpenSimplex in our codebase - can we port?)
- [ ] Avoiding texture lookups (compute-only noise)
- [ ] Trade-offs: quality vs performance

### Buffer Access Patterns
- [ ] Coalesced memory access (why it matters)
- [ ] Strided access penalties
- [ ] Vertex buffer layout (`Vertex` struct with position, normal, color)
- [ ] Updating subset of struct fields (Y position only)

**Learning doc**: `learnings/wgsl_compute_patterns.md`

---

## Priority 2: wgpu Rust Integration (PRACTICAL)

**Target**: 0.5 session

### Pipeline Creation
- [ ] `ShaderModule` creation from WGSL source
- [ ] `ComputePipeline` descriptor fields
- [ ] `BindGroupLayout` for compute shaders
- [ ] `BindGroup` creation (mapping buffers to bindings)
- [ ] Pipeline layout (`PipelineLayout` with bind group layouts)

### Buffer Management
- [ ] `BufferUsages` flags: `STORAGE`, `VERTEX`, `COPY_DST`
- [ ] Why we need `STORAGE | VERTEX` for compute-updated vertex buffers
- [ ] `create_buffer_init` vs `create_buffer` (initialization strategies)
- [ ] `queue.write_buffer()` for uniform updates
- [ ] Buffer size alignment requirements

### Compute Pass Execution
- [ ] Command encoder creation
- [ ] `begin_compute_pass()` descriptor
- [ ] Setting pipeline and bind groups
- [ ] Dispatch calculation: `(vertex_count + workgroup_size - 1) / workgroup_size`
- [ ] Timestamp queries (profiling GPU time)

### Synchronization
- [ ] When compute results are visible to render pipeline
- [ ] Implicit barriers between passes
- [ ] `queue.submit()` ordering guarantees
- [ ] CPU-GPU sync points (`device.poll()`, async mapping)

**Learning doc**: `learnings/wgpu_compute_integration.md`

---

## Priority 3: Performance Optimization (ADVANCED) - DEFER

**Target**: After initial integration works

### Workgroup Sizing
- [ ] Hardware limits (M1 GPU specs: threads per SIMD group, occupancy)
- [ ] Power-of-2 sizes (64, 128, 256, 512)
- [ ] Benchmarking different sizes for ocean mesh workload
- [ ] Trade-offs: occupancy vs shared memory usage

### Memory Access Optimization
- [ ] Bank conflicts in shared memory
- [ ] Cache line utilization
- [ ] Prefetching strategies
- [ ] Vertex buffer structure layout (AoS vs SoA)

### Profiling Tools
- [ ] Xcode Instruments GPU profiling (Metal backend)
- [ ] wgpu timestamp queries
- [ ] Frame capture and analysis
- [ ] Identifying bottlenecks (ALU-bound vs memory-bound)

**Defer until**: Initial compute pipeline integrated and working

---

## Priority 4: Advanced Techniques (SPECIALIZED) - DEFER

**Target**: Future features (FFT synthesis, flowfield)

### FFT on GPU
- [ ] Radix-2 Cooley-Tukey algorithm
- [ ] In-place computation with shared memory
- [ ] Twiddle factor precomputation
- [ ] Multi-pass FFT for large sizes

### Texture Sampling in Compute
- [ ] When to use textures vs storage buffers
- [ ] Sampler configuration
- [ ] Filtering and interpolation
- [ ] Texture atlas strategies for procedural data

### Indirect Dispatch
- [ ] GPU-driven workgroup counts
- [ ] Conditional execution (skip work on GPU)
- [ ] DrawIndirect and DispatchIndirect buffers

**Defer until**: Ocean compute stable, ready for advanced features

---

## Research Sources

### Official Documentation
- [ ] [WGSL Specification](https://www.w3.org/TR/WGSL/) - Language reference
- [ ] [WebGPU Specification](https://www.w3.org/TR/webgpu/) - API surface
- [ ] [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/) - Rust bindings

### Tutorials & Guides
- [ ] [Learn wgpu](https://sotrh.github.io/learn-wgpu/) - Comprehensive wgpu tutorial
- [ ] [WebGPU Fundamentals](https://webgpufundamentals.org/) - Compute shader examples
- [ ] [GPU Gems articles](https://developer.nvidia.com/gpugems/gpugems3/part-vi-gpu-computing) - Classic GPU compute patterns

### Platform-Specific
- [ ] M1 GPU architecture (Apple Silicon GPU overview)
- [ ] Metal Performance Shaders (reference for optimal patterns on Apple hardware)
- [ ] wgpu Metal backend specifics (how WGSL maps to Metal Shading Language)

### Code Examples
- [ ] wgpu examples repository (compute shader examples)
- [ ] Bevy compute examples (if relevant for game engine context)
- [ ] Our `ocean_compute.wgsl` (annotate with learnings)

---

## Open Questions (Pre-Study)

**Execution Model:**
1. Why exactly 256 threads per workgroup? (Hardware SIMD width? Occupancy sweet spot?)
2. What happens if `vertex_count % 256 != 0`? (Bounds checking needed?)
3. Can we dispatch multiple workgroups in 2D/3D grid? (Spatial locality benefits?)

**Memory:**
4. What's the actual uniform buffer size limit on M1? (Docs say 64KB, is this per binding?)
5. Does alignment matter for performance or just correctness? (Cache line boundaries?)
6. Can we use shared memory for noise computation? (Intermediate gradient storage?)

**Integration:**
7. How do we profile GPU time separately from CPU time? (Timestamp queries?)
8. What's the synchronization overhead between compute and render passes? (Barrier cost?)
9. Should we double-buffer vertex data? (Avoid read-write hazards?)

**Correctness:**
10. How to validate compute shader output matches CPU reference? (Tolerance for float precision?)
11. What debugging tools exist for WGSL? (Print debugging? Renderdoc support?)
12. How to handle shader compilation errors gracefully? (Validation layer output?)

---

## Meta-Tracking Strategy

After each priority:
- Create learning doc with key insights
- Update open questions (answered vs new)
- Self-assess: ready for next priority?

Assessment milestones:
- `learnings/.ddd/0_pre_study_questions.md` - This list
- `learnings/.ddd/1_compute_foundations.md` - After Priority 0-1
- `learnings/.ddd/2_integration_ready.md` - After Priority 2

---

## Time Box

**Total research budget**: 1.5 sessions (Priorities 0-2)
- Priority 0: 0.5 session (foundations)
- Priority 1: 0.5 session (patterns)
- Priority 2: 0.5 session (wgpu integration)

**Exit condition**: Can confidently integrate compute pipeline from HANDOFF.md with understanding of:
- Why each line of code exists
- How to debug if it doesn't work
- Where to optimize if performance isn't as expected

**Warning signs to stop researching**:
- Diminishing returns (reading similar explanations)
- Analysis paralysis (afraid to touch code)
- Tangent diving (studying FFT before basic compute works)

---

## Transition to Discovery

After research complete:
- Build toy compute shader (simple addition, verify execution)
- Integrate ocean compute (follow HANDOFF.md with understanding)
- Profile before/after (validate 15-30x gain hypothesis)
- Document learnings (what worked, what surprised us, what to optimize)

Research → Discovery → Learnings → README cycle continues.

---

**Remember**: Good enough to start building > perfect understanding. Practice reveals gaps theory can't predict.
