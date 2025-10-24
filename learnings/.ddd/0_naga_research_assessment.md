# PHASE 0 — Naga Research Assessment

**Date**: October 2025
**Phase**: Research mode study (Priorities 0-2)
**Status**: Complete
**Next**: Transition to Discovery mode for practical validation

---

## What Was Studied

### Sources Consulted

**Primary documentation** (cached to `.webcache/naga/`):
- `docs.rs/naga/latest/naga/` - Main API documentation
- `naga/README.md` - Project overview and supported languages
- `naga::valid` module docs - Validator API and error types
- `naga::back::spv` module docs - SPIR-V backend
- `naga::back::msl` module docs - MSL backend
- `wgpu` main docs - Integration points

**Coverage**:
- ✅ Priority 0: Foundational architecture (0.5 session)
- ✅ Priority 1: Validation system (0.5 session)
- ✅ Priority 2: Translation backends (0.5 session)
- ⏭️ Priority 3: Optimization & IR (deferred to post-Discovery)
- ⏭️ Priority 4: Integration patterns (deferred)

**Total research time**: ~1.5 hours (as planned)

---

## Key Insights (Not Just Facts)

### Insight 1: Naga Is Not Optional in wgpu Stack

**What I learned**: Every wgpu shader goes through Naga, whether you call it directly or not.

**Implication**: Understanding Naga errors is critical for debugging shader issues in vibesurfer. When `device.create_shader_module()` fails, the error originates from Naga validation or backend translation.

**Actionable**: vibesurfer shader debugging workflow should include:
1. Test with `naga-cli` directly (isolate Naga vs wgpu issues)
2. Check validation errors first (types, bindings)
3. If validation passes but wgpu fails, check backend translation (MSL on macOS)

### Insight 2: Validation is Cheap, Mandatory, and Not Actually Optional

**What I learned**: Backends require `ModuleInfo` from validation. Even if you want to skip validation, you can't (will panic).

**Implication**: "Should I validate at runtime?" is the wrong question. The right question is "What validation flags and capabilities should I use?"

**Actionable**:
- Runtime validation is fine (< 1ms cost)
- Focus on using correct capabilities for target platform
- Build-time validation is for catching errors early, not avoiding runtime cost

### Insight 3: Backend Fidelity Varies in Non-Obvious Ways

**What I learned**: Module can pass validation but fail backend translation. Capabilities in validator don't guarantee backend support.

**Example**: Int64 atomics pass validation with `Capabilities::all()`, but MSL backend fails (Metal doesn't support).

**Implication**: Platform-specific validation is more important than I initially thought. Validating with `Capabilities::all()` is convenient but hides platform incompatibilities.

**Actionable**: vibesurfer should validate with Metal-specific capabilities (not `Capabilities::all()`), or handle backend errors gracefully.

### Insight 4: Binding Model Translation is Where Complexity Hides

**What I learned**: WGSL has `@group(X) @binding(Y)` (two-level hierarchy). Vulkan maps this directly to descriptor sets. Metal has flat indices. DX has register spaces.

**Implication**: wgpu handles this mapping internally, but if we ever use Naga directly or debug binding issues, we need to understand the translation.

**Vibesurfer relevance**: On macOS, our WGSL `@group/@binding` gets flattened to Metal argument buffer indices. If we see binding errors, this is likely the source.

### Insight 5: "Shader Translation Library" Undersells What Naga Does

**What I learned**: Naga is not just translation. It's:
- Type-checking system (validation)
- IR optimization layer (proc module)
- Platform compatibility checker (capabilities)
- Source location tracker (spans for error reporting)

**Implication**: Naga is infrastructure for shader tooling, not just a compiler backend. We can build linters, formatters, analyzers on top of Naga IR.

**Future opportunity**: vibesurfer could have build-time shader linting (check for common mistakes, enforce conventions).

---

## Questions Answered (From Research Plan)

### From Priority 0

**Q: What does Naga do vs what does wgpu do?**
- **Naga**: Parse, validate, translate shaders (standalone library)
- **wgpu**: Graphics API (device, queue, shaders, pipelines). Uses Naga internally for shader processing.

**Q: When should you use Naga directly vs via wgpu?**
- **Via wgpu**: Normal app development (wgpu handles Naga automatically)
- **Direct**: Shader tooling (linters, validators, offline translation), build-time checks, custom workflows

**Q: What's the relationship between Naga IR and SPIR-V?**
- **SPIR-V**: Binary IR for Vulkan/OpenCL (low-level, driver-ready)
- **Naga IR**: Rust-native IR (higher-level, easier to analyze/transform)
- Naga can parse SPIR-V → Naga IR, and emit Naga IR → SPIR-V
- Not 1:1 mapping (Naga IR is higher abstraction)

### From Priority 1

**Q: How does Naga parse WGSL into IR?**
- `naga::front::wgsl::parse_str()` → `Module` (IR)
- Recursive descent parser (syntax) + semantic analysis (types, scopes)
- Errors include spans (source locations) for debugging

**Q: What validation does Naga perform?**
- Type correctness, control flow, resource bindings, capability checks, memory layouts
- Configured via `ValidationFlags` (which checks) and `Capabilities` (which features allowed)

**Q: What errors come from Naga vs driver/wgpu?**
- **Naga parse errors**: Syntax/semantic issues in source
- **Naga validation errors**: IR correctness (types, bindings, control flow)
- **Naga backend errors**: Platform incompatibility (feature not supported by target)
- **Driver errors**: GPU driver rejected shader (happens during `vkCreateShaderModule`, `MTLLibrary` creation)

**Q: How to get useful error messages from Naga?**
- Naga errors include `WithSpan` wrapper (source locations)
- Use `codespan-reporting` for pretty-printed errors
- `naga-cli` provides formatted error output

### From Priority 2

**Q: How to translate WGSL → SPIR-V?**
- Parse → Validate → `spv::write_vec()` → binary (Vec<u32>)
- Options control SPIR-V version, debug info, bounds checks

**Q: How to translate WGSL → MSL?**
- Parse → Validate → `msl::Writer::new().write()` → text (String)
- Options control MSL version, binding mappings, workgroup memory init

**Q: What gets lost in translation?**
- Variable names (unless debug info enabled)
- Comments (not part of IR)
- Precision (may be lowered, e.g., f32 → half on mobile)
- Platform-specific: Int64 atomics (Metal), f64 (mobile GPUs)

**Q: How to control translation options?**
- Backend-specific `Options` structs (`spv::Options`, `msl::Options`, etc.)
- Language version, flags, bounds checking, binding maps, zero-init policies

---

## Questions Raised (Theory vs Practice Gaps)

### Q1: What do vibesurfer's actual validation errors look like?

**Theory**: Errors have types (`ValidationError::Type`, etc.) and spans
**Practice gap**: Need to trigger validation failure in real vibesurfer shader
**Validation needed**: Intentionally break shader, inspect wgpu error message, trace to Naga

### Q2: What does vibesurfer WGSL translate to on macOS?

**Theory**: WGSL → Naga IR → MSL (Metal 2.x)
**Practice gap**: Haven't inspected actual MSL output for vibesurfer shaders
**Validation needed**: Use `naga-cli` to translate compute/fragment shaders, inspect MSL

### Q3: What are actual validation + translation costs for vibesurfer?

**Theory**: Parse + validate + translate ~1-3ms for typical shaders
**Practice gap**: No measurements on vibesurfer's actual shaders (compute: sphere projection, terrain gen; fragment: wireframe, fog)
**Validation needed**: Benchmark Naga pipeline on real shaders

### Q4: Do vibesurfer shaders use platform-incompatible features?

**Theory**: Should validate with Metal capabilities, not `Capabilities::all()`
**Practice gap**: Don't know if current shaders would fail with conservative capabilities
**Validation needed**: Test validation with `Capabilities::default()`, check for errors

### Q5: Can vibesurfer benefit from build-time shader validation?

**Theory**: `build.rs` can validate shaders, catch errors before runtime
**Practice gap**: Don't know if vibesurfer shader errors are common enough to warrant
**Validation needed**: Measure how often shader changes break validation (track over time)

### Q6: What does naga-cli output look like for vibesurfer shaders?

**Theory**: `naga shader.wgsl shader.metal` translates and validates
**Practice gap**: Haven't tested on actual vibesurfer shaders
**Validation needed**: Run naga-cli on `vibesurfer/src/shaders/*.wgsl` (or toy4 shaders)

---

## Decisions Made

### Decision 1: Research Priorities 0-2, Defer 3-4

**Rationale**:
- Priorities 0-2 answer core questions (architecture, validation, translation)
- Priority 3 (optimization/IR) needs practical context (what to optimize?)
- Priority 4 (integration) is vibesurfer-specific (not general reference)

**Result**: Research phase complete with foundational understanding. Ready to validate with toy.

### Decision 2: Create General-Purpose Reference, Not Vibesurfer-Specific

**Rationale**:
- Naga learnings apply to any Rust project with shaders
- Infrastructure compounds (save tokens across all future shader work)
- Vibesurfer is validation playground, not the product

**Result**: `learnings/naga-reference.md` is reusable. Vibesurfer-specific learnings will be in Discovery phase.

### Decision 3: Cache External Sources, Don't Transcribe

**Rationale**:
- External docs exist (don't duplicate)
- Synthesis > transcription (extract patterns, not copy-paste)
- Cache for offline access, link in attribution

**Result**: `.webcache/naga/` has source material. Learning docs have synthesized insights.

### Decision 4: Structure Learnings as Hierarchical (Architecture → Validation → Translation)

**Rationale**:
- Learning progression: foundational → essential → practical
- Matches agent mental model (understand system before using it)
- Enables skipping to relevant section (don't need to read all 3 docs)

**Result**: 3 focused docs + 1 comprehensive reference. Choose depth vs breadth.

---

## What Was Learned (Synthesized)

### Core Mental Model

**Naga = Shader compiler middle-end**
- Frontend (parse) → IR → Validator (check) → Backend (translate)
- Used standalone or via wgpu
- IR is Rust-native (not SPIR-V), enables analysis/transformation

### Validation Model

**Mandatory, cheap, configurable**
- Backends require `ModuleInfo` (can't skip)
- Cost is negligible (< 1ms for typical shaders)
- Configure via flags (which checks) and capabilities (which features)

### Translation Model

**Platform differences matter**
- Binding models differ (descriptor sets vs flat indices vs registers)
- Coordinate systems differ (Y-up vs Y-down)
- Feature availability differs (f64, int64 atomics, subgroups)
- Validate with target capabilities to catch incompatibilities early

### Error Model

**Hierarchical, spanned, debuggable**
- Errors include source locations (spans)
- Naga errors ≠ driver errors (different sources)
- Use `naga-cli` for offline debugging

---

## What Remains Uncertain (Needs Discovery)

### Uncertainty 1: Practical Error Patterns

**What we don't know**: Common mistakes when writing WGSL for vibesurfer
- Do binding errors happen often?
- Are type errors obvious or subtle?
- Do backend translation failures occur?

**How to resolve**: Build toy, intentionally trigger errors, catalog patterns

### Uncertainty 2: Real-World Translation Output

**What we don't know**: What MSL does Naga generate for vibesurfer shaders?
- Is it readable?
- Does it match hand-written MSL?
- Are there inefficiencies?

**How to resolve**: Translate vibesurfer shaders with `naga-cli`, inspect MSL

### Uncertainty 3: Performance in Practice

**What we don't know**: Actual Naga pipeline cost for vibesurfer
- Is 1ms accurate for our shaders?
- Does validation show up in profiling?
- Should we cache translated shaders?

**How to resolve**: Benchmark parse + validate + translate on real shaders

### Uncertainty 4: Build-Time Validation Value

**What we don't know**: Would build-time shader validation help vibesurfer development?
- How often do shader changes break validation?
- Would catching errors at build time save debugging time?
- Is it worth adding to `build.rs`?

**How to resolve**: Track shader validation failures over development sessions

### Uncertainty 5: Platform Capability Constraints

**What we don't know**: Do vibesurfer shaders rely on features not in conservative Metal capabilities?
- Would `Capabilities::default()` reject our shaders?
- Do we use any Metal-specific extensions?
- Are there cross-platform compatibility issues?

**How to resolve**: Test validation with platform-specific capabilities

---

## Next Steps

### Transition to Discovery Mode

**Goal**: Build toy to validate research learnings

**Toy scope** (minimal, focused):
1. Parse vibesurfer shader with Naga (compute or fragment)
2. Validate with different capability sets (all vs default vs Metal)
3. Translate to MSL, inspect output
4. Intentionally break shader, trigger validation errors, inspect messages
5. Benchmark parse + validate + translate pipeline

**Deliverables**:
- Toy code (`toys/naga_exploration/`)
- Discovery LEARNINGS.md (what worked, what didn't, measurements)
- Updated naga-reference.md with practical insights

### Open Questions for Discovery

**Test with real shaders**:
- Q1: What validation errors occur with vibesurfer shaders?
- Q2: What does MSL output look like?
- Q3: What's the actual performance cost?

**Test platform compatibility**:
- Q4: Do shaders work with conservative capabilities?
- Q5: Are there Metal-specific quirks?

**Test tooling**:
- Q6: Is `naga-cli` useful for vibesurfer workflow?
- Q7: Should we add build-time validation?

### Success Criteria

**Discovery phase complete when**:
- ✅ Validated real vibesurfer shader with Naga standalone
- ✅ Measured parse + validate + translate performance
- ✅ Inspected MSL translation output
- ✅ Triggered and cataloged validation errors
- ✅ Tested with platform-specific capabilities
- ✅ Updated naga-reference.md with practical findings

**Ready to close Naga learning cycle when**:
- Theory (research docs) ✅
- Practice (Discovery toy) ⏳
- Synthesis (updated reference) ⏳

---

## Meta-Learning: What This Research Revealed About DDD

### Pattern: External Knowledge Requires Structured Exploration

**What worked**:
- Priority ordering (foundational → essential → practical)
- Webcache protocol (offline access, attribution)
- Synthesis focus (patterns > facts)

**What didn't**:
- Initial attempt to use WebFetch (interrupted, corrected to webcache)
- Learned: webcache is the protocol, not WebFetch

### Pattern: Reference Docs Compound Across Projects

**Insight**: This Naga reference isn't just for vibesurfer. It's infrastructure.

**Value**: Every future Rust + shader project saves 1.5 hours of re-research
- Toy experiments with compute shaders
- Graphics demos
- Shader tooling projects

**Implication**: "Artifacts disposable, clarity durable" in action. This reference outlasts vibesurfer.

### Pattern: Research → Discovery Cycle Works

**Research phase** (this):
- Gathered external knowledge
- Built mental model
- Identified practice gaps

**Discovery phase** (next):
- Validate theory with practice
- Measure real behavior
- Update model with findings

**Synthesis**:
- Update reference with practical insights
- Close gaps between theory and practice
- Ready for next cycle (if needed)

---

## Reflection

**What surprised me**:
- Validation is truly mandatory (not optional with workarounds)
- Backend fidelity varies more than expected (validation passing ≠ translation succeeding)
- Binding model translation is where platform complexity hides

**What confirmed expectations**:
- Naga is well-designed (clean architecture, good error messages)
- wgpu integration is seamless (most users never touch Naga directly)
- Documentation is comprehensive (docs.rs has what we need)

**What changed my mental model**:
- Before: "Naga is a shader compiler"
- After: "Naga is shader infrastructure (compiler + type checker + platform abstraction + tooling foundation)"

**What I'm excited to validate**:
- See actual MSL output for vibesurfer shaders
- Measure real performance (confirm < 1ms claim)
- Test validation errors (see if spans help debugging)

---

## Conclusion

Research phase successful. Three learning documents + one comprehensive reference created. Core questions answered. Practice gaps identified.

Ready to transition to Discovery mode: build toy to validate learnings with real vibesurfer shaders.

**Artifacts produced**:
- `learnings/naga-architecture.md` (Priority 0)
- `learnings/naga-validation.md` (Priority 1)
- `learnings/naga-translation.md` (Priority 2)
- `learnings/naga-reference.md` (Synthesis)
- `learnings/.ddd/0_naga_research_assessment.md` (This)
- `.webcache/naga/*` (Cached sources)

**Next**: `hegel start discovery` → Build toy → Validate theory → Update reference → Close cycle
