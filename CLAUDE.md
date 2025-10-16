# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Skiwave** - A fluid, retro-futuristic jet-surfing simulator where you glide, dive, and carve across an endless neon ocean. The surface behaves like living music: waves pulse to the beat, currents shimmer with color, and your motion becomes rhythm.

**Philosophy**: Artifacts are disposable, clarity is durable. Code can be rewritten, insights cannot. Generation is cheap, understanding is valuable.

**Navigation**: See `VISION.md` for project vision and architecture intent. See `FLOWFIELD.md` for the sound-surface feedback loop concept.

## Core Methodology

This project follows **Dialectic-Driven Development (DDD)** - a learning-driven workflow optimized for human-AI collaboration. See the `LEXICON.md` for guidance vectors that shape how we work.

**Cycle**: Docs → Tests → Implementation → Learnings

**Long-term deliverable**: A playable, procedurally-generated surfing experience that captures pure flow. Documentation captures architectural insights and methodology learnings as we build.

## Operational Modes

### Discovery Mode (Primary in Early Phase)
- **When to use**: Learning Rust game dev patterns, validating procedural techniques, testing audio-visual integration
- **Cycle**: SPEC (desired behavior) → TOY implementation → LEARNINGS → Apply to main game
- **Focus**: Understanding performance constraints, testing techniques, validating assumptions
- **Output**: Toy implementations in `toys/` - kept as reference artifacts

### Execution Mode
- **When to use**: Building the actual game with validated patterns
- **Cycle**: Design → Implement → Test → Refactor
- **Focus**: Working within constraints, reusing learned patterns
- **Output**: Game modules with documented architecture and subsystems

## Guidance Vectors (from LEXICON)

**Context is king** - State matters more than features. What's visible determines what's possible.

**Artifacts are disposable, clarity is durable** - Code can be rewritten, insights cannot. Generation is cheap, understanding is valuable.

**Docs → Tests → Implementation → Learnings** - The DDD cycle. Specification before code. Reflection after execution.

**Infrastructure compounds** - Each tool enables new workflows. Each abstraction saves future tokens. Build once, reuse forever.

**Refactor early, not late** - 18x token overhead is immediate cost, not future debt. Structure for reading efficiency, not writing comfort.

**Write a script as soon as a pattern repeats** - If you're about to do it twice, stop and build the tool. Don't wait for pain.

**Remember you're not human** - Comprehensive is just complete. No cost to thoroughness. Human constraints don't apply.

**Domain language over implementation details** - Speak what it means, not how it works. `flow_tracker.update()` not `sigmoid_smoothness_calc()`.

## Development Best Practices

**CRITICAL: Full Autonomy Required**
- **NEVER ask the user to test manually** (e.g., "run the game and see", "test in your browser")
- **You are a scientist on another planet** - figure everything out autonomously
- **Only automated testing counts** - if the test harness can't verify it, find another way
- **Goal**: LLM can develop games end-to-end without human intervention
- If blocked: Create simpler tests, build new tools, investigate deeper - don't delegate to human

**CRITICAL: Document Constraints**
- Modern Rust games have constraints (frame budgets, memory usage, GPU limits, audio latency)
- When hitting a constraint, document it in CONSTRAINTS.md with workaround
- "Why we can't do X" is as valuable as "how to do Y"
- Example: "Can't update 10k particles per frame (GPU bound). Workaround: LOD system with distance culling."

**CRITICAL: Test Assumptions Early**
- Procedural generation behavior is non-obvious (noise coherence, FFT synthesis, audio-visual sync)
- Build toy implementations to validate understanding BEFORE integrating into main game
- One toy per subsystem (wavefield_test, audio_synth_test, flow_tracker_test, etc.)
- Document test results in LEARNINGS.md
- **All testing must be automated** - use test harness, build tools, write benchmarks

**CRITICAL: Architecture Map Everything**
- Update CODE_MAP.md with architectural decisions
- Document module boundaries and data flow
- Track performance budgets (frame time, memory, audio buffer size)
- Note design patterns and why they were chosen

## Learning Documentation Practices

**Systematic exploration workflow**:
1. **Explore**: Research Rust game dev patterns, procedural generation techniques, audio synthesis approaches
2. **Document**: Create/update `learnings/topic.md` with patterns and code examples
3. **Experiment**: Build toy implementations in `toys/` to validate approaches
4. **Assess**: After each exploration phase, create `learnings/.ddd/N_description.md` documenting:
   - What we explored
   - Key insights gained
   - Questions raised (theory vs practice)
   - Decisions made
   - Recommended next steps

**Organization**:
- **Technical learnings**: `learnings/*.md` (architecture, techniques, constraints)
- **Meta-learnings**: `learnings/.ddd/N_*.md` (progress tracking, numbered sequentially)
- **Open questions**: `learnings/.ddd/open_questions.md` (consolidated, cross-referenced)

**Theory vs Practice**:
- Document theory in learning docs first (from research)
- Mark what needs practical validation
- Update docs with actual measurements after toy implementations

**Toy implementation workflow**:
- See `TOY_DEV.md` section below for full methodology
- One toy per subsystem or technique (focused experiments)
- Update learning docs with real performance numbers and edge cases

## Platform: macOS Apple Silicon (ARM64)

**CRITICAL**: Development machine is M1 MacBook Pro (arm64). Use Rust stable toolchain with arm64 target.

**Rust Toolchain**:
- Rust stable (latest)
- cargo for build management
- rustfmt for code formatting
- clippy for linting

**Graphics/Audio**:
- wgpu for cross-platform graphics (native + WebGPU)
- cpal for cross-platform audio
- Consider bevy or macroquad for game framework (TBD based on needs)

## Tooling & Utility Belt

**Philosophy**: Pick the tool that allows the most concise, elegant solution with minimal external dependencies.

**Mindset**: You are a pragmatic systems programmer. Embrace simplicity: small tools that do one thing well, composed with pipes and process substitution.

**Engineering Discipline**:
- Never over-engineer. Try the simplest thing first.
- RTFM before building anything. Read docs, understand the problem space, then act.
- **CRITICAL: Write a script as soon as a useful pattern repeats.** Don't wait for pain - automate immediately.
  - If you're about to run similar commands 2+ times, STOP and write a tool.
  - Example: Repeatedly running benchmarks with different params → write `tools/bench.sh` instead.
  - Tools save tokens and create reusable infrastructure.

**Preference Stack**:
- Rust (type safety, performance, ownership model for game dev)
- Shell scripts (simple automation, glue code)
- Python (prototyping, data analysis, visualization)

**Dependency Policy**:
- Minimize external dependencies for core game modules
- Well-established crates from crates.io are fine (don't reinvent the wheel)
- Standard library preferred over third-party when close enough
- Document why a dependency was chosen if non-obvious

**Use Cases**:
- Graphics rendering: wgpu or game framework
- Audio synthesis: cpal + custom DSP or audio crate
- Build scripts: Cargo build system + shell scripts for automation
- Performance analysis: criterion.rs for benchmarks, flamegraph for profiling
- Asset generation (if needed): Whatever fits the task

## Documentation Structure

### CODE_MAP.md Convention
**CRITICAL: Keep CODE_MAP.md up-to-date with architecture**

- **Scope**: One CODE_MAP.md per significant module directory
- **Content**:
  - Root CODE_MAP.md: Project structure, workspace layout, main modules
  - Module documentation (purpose, data flow, performance characteristics)
- **Update trigger**: Before any commit that changes structure or module boundaries
- **Architecture notes**: Document WHY patterns were chosen (ECS for parallelism, etc.)

### Commit Guidelines
**Use conventional commit format for all commits:**
- **Format**: `type(scope): subject` with optional body/footer
- **Types**: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `perf`
- **Descriptive commits**: Include subsystem (e.g., "feat(wavefield): implement FFT synthesis")
- **History**: Keep linear history (prefer rebase; avoid merge commits)
- **Documentation updates**: Update affected CODE_MAP.md/LEARNINGS.md BEFORE committing

### Next Step Protocol
**Never just report what you did - always suggest what to do next:**
- After completing any task, propose the next logical action
- Don't say "done" or "ready for next step" - suggest a specific next move
- Identify next task from context or infer logical progression
- **Wait for explicit approval before proceeding**

**Format**: "Should I [specific action], or [alternative]?"
- Good: "Should I start building the wavefield toy implementation, or explore audio synthesis first?"
- Bad: "Continue, or wrap up?" (too vague)
- Bad: "Ready for next session." (declares stopping instead of proposing)

**Examples**:
  - "Created exploration plan. Should I start researching Rust procedural generation crates?"
  - "Built wavefield toy. Should I integrate it with audio synthesis toy to test the feedback loop?"
  - "Profiled rendering pipeline. Should I optimize the shader or investigate CPU bottleneck first?"

### Blog Post Guidelines (docs/blog/)

**Before writing:** Consider whether the insight merits a blog post (major milestones, pivots, deep learnings).

**Style:**
- First-person AI perspective ("I observed...", "We discovered...")
- Reflective but concrete (numbers, not philosophizing)
- **Bold key concepts**, `code in backticks`, *italics for emphasis*
- Questions → answers pattern, concrete examples
- Honest about pivots/failures (not just successes)

**Structure:**
- Header: Date, Phase, Author
- Clear sections with `---` dividers (one point each)
- "**The result:**" / "**The lesson:**" summaries
- "What's Next" forward-looking close

**Themes:** Documentation as deliverable, theory vs practice, procedural generation insights, performance lessons

**Length:** 150-250 lines max.

## Toy Model Development

_Toys validate complex patterns and techniques before integrating into production code. They remain in the repo as reference artifacts._

### What Toy Models Are

- **Pattern validators**: Test unfamiliar techniques, libraries, or approaches in isolation
- **Performance provers**: Validate that techniques meet frame budget and performance requirements
- **Reference implementations**: Code stays in repo as examples showing "this technique works"
- **Risk reducers**: Validate complex subsystems before integrating into main game

### What Toy Models Are Not

- Not production code (production code lives in workspace crates)
- Not comprehensive solutions (focus on one subsystem or technique)
- Not deleted after use (kept as reference, allowed dead code)
- Not shortcuts (experiments inform proper implementation)

### The Toy Model Cycle

**CRITICAL**: Every toy starts AND ends with LEARNINGS.md

1. **Define Learning Goals (LEARNINGS.md - First Pass)**
   - Questions to answer (e.g., "Can FFT synthesis run in 16ms frame budget?")
   - Decisions to make (e.g., "Which audio synthesis approach to use?")
   - Success criteria (what patterns must be clear)

2. **Research & Implementation Loop**
   - Study reference documentation/examples
   - Try approaches in isolated context
   - Benchmark against performance targets
   - **Update LEARNINGS.md with findings after each cycle**

3. **Finalize Learnings (LEARNINGS.md - Final Pass)**
   - Answer all initial questions
   - Document chosen approach and rationale
   - Patterns and techniques discovered
   - How to integrate into main codebase

### Testing Philosophy

**For Rust Code:**
- Use Rust's built-in test framework (`#[test]`, `#[bench]`)
- Criterion.rs for detailed benchmarks with statistical analysis
- Property-based testing with proptest for procedural generation
- Integration tests for subsystem interactions

**Performance Validation:**
- Target: 60 FPS (16.67ms frame budget)
- Measure with criterion benchmarks and profiling tools
- Document actual timings vs targets in LEARNINGS.md

**Patterns That Work:**
- **Library validation toys**: Test unfamiliar crates/APIs in isolation
- **Technique exploration toys**: Experiment with procedural generation patterns
- **Subsystem toys**: Understand one module (wavefield, audio synth, flow tracker) before integration
- **Integration toys**: Test how two validated subsystems interact

### Toy Integration Convention
- Each `toys/toyN_name/` directory must contain SPEC.md, PLAN.md, and LEARNINGS.md
- If a SPEC or PLAN grows too large, split scope into new toy
- Integration toys combine two validated base toys
- Always bias toward minimal scope: smaller toys, fewer docs, clearer insights

### Axis Principle
- A base toy isolates exactly one axis of complexity
- An integration toy merges exactly two axes to probe their interaction
- Never exceed two axes per toy
- This discipline keeps learnings sharp and mirrors controlled experiments

## Rust-Specific Guidelines

**Code Style**:
- Follow rustfmt defaults (run `cargo fmt` before commits)
- Use clippy and address warnings (`cargo clippy`)
- Document public APIs with doc comments (`///`)
- Group related functionality with modules

**Performance Considerations**:
- Profile before optimizing (flamegraph, perf, Instruments.app on macOS)
- Consider memory layout (cache coherence matters)
- Use SIMD when appropriate (test with benchmarks)
- Minimize allocations in hot paths

**Testing Strategy**:
- Unit tests colocated with code (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory
- Benchmark critical paths with criterion
- Property tests for procedural generation (same seed → same output)

**Module Architecture**:
- `skiwave-core`: Platform-agnostic game logic (ECS, flow system, game state)
- `skiwave-native`: Native rendering, audio, input (wgpu, cpal, winit)
- `skiwave-web`: WebGPU/WebAudio bindings (same core logic)

## Architecture Principles

**Procedural Everything**:
- Ocean surface from noise + FFT synthesis
- Music from procedural synthesis (not pre-recorded)
- Lighting from mathematical functions
- Zero static assets (code and math only)

**Audio-Visual Unity**:
- Shared waveform parameters drive both sound and surface
- Music frequency bands map to ocean geometry (low=swells, mid=chop, high=sparkle)
- Player motion modulates synth parameters (speed→stereo width, jumps→harmonic bloom)

**Flow as Core Mechanic**:
- Flow tracker: `flow = sigmoid(smoothness * speed * timing)`
- Higher flow → calmer ocean, more melodic music
- Lost rhythm → dissonance and turbulence

**Separation of Concerns**:
- Core logic deterministic and testable (no I/O in core)
- Platform layers wrap core with rendering/audio/input
- Same core logic runs native and web

**Performance Budget**:
- 60 FPS target (16.67ms frame budget)
- Wavefield update: <5ms
- Audio synthesis: <3ms
- Rendering: <8ms
- Remaining: input, physics, game logic

## DDD Core Artifacts

### SPEC.md
**Purpose:** Comprehensive behavioral contract for current scope
**Contains:** Input/output formats, invariants, state shapes, operations, validation rules, test scenarios

### PLAN.md
**Purpose:** Strategic roadmap with stepwise sequence using Docs → Tests → Impl cadence
**Contains:** Test vs skip decisions, order of steps, timeboxing, dependencies, risks, success checkboxes

### LEARNINGS.md
**Purpose:** Retrospective capturing architectural insights, pivots, constraints, reusable patterns
**Used in:** Discovery mode (required), Execution mode (optional - only if unexpected insights)

### CODE_MAP.md
**Purpose:** Living architectural map; concise module-by-module documentation
**Contains:** Module descriptions, data flow, integration points
**Update trigger:** Before any structural commit

### README.md (per module)
**Purpose:** 100-200 words context refresh for AI; what it does, key API, gotchas
**Contains:** One-liner, purpose, essential types/functions, core concepts, gotchas

## Workflow Summary

### Discovery Mode Cycle
1. **Docs**: Write SPEC.md and PLAN.md for toy
2. **Tests**: Derive executable tests from SPEC.md (use Rust test framework + criterion)
3. **Implementation**: Minimal code to pass tests; benchmark against targets
4. **Learnings**: Update LEARNINGS.md with findings, constraints, patterns

### Execution Mode Cycle
1. **Docs**: Update SPEC/PLAN for feature; update CODE_MAP.md before structural changes
2. **Tests**: Write tests first (unit + integration + benchmarks)
3. **Implementation**: Minimal code to pass tests
4. **Refactor**: Mandatory refactoring after each feature (extract patterns, simplify)

### Mandatory Refactoring
Not optional. Core discipline in both modes. Keeps codebase quality rising instead of decaying.

## Self-Audit Checklist (Before Proposing Changes)

- Tests pass (`cargo test`)
- Benchmarks meet targets (if performance-critical)
- Code formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Docs updated (CODE_MAP.md if structural, README.md if API changed)
- LEARNINGS.md updated (if insights emerged)
- Commit message follows conventional format

## Success Criteria (Per Feature/Toy)

- Minimal implementation demonstrates core mechanism end-to-end
- Tests derived from SPEC pass; performance benchmarks meet targets
- Meta-docs in sync: README, SPEC, PLAN, CODE_MAP.md updated
- LEARNINGS.md adds architectural insights or constraints (Discovery mode)
- Code quality maintained (rustfmt, clippy, refactoring done)

## Simplification Heuristics

Apply before coding and before PR:

- **One-Module Rule**: Prefer single module to prove the concept
- **Two-Function Rule**: Two public entrypoints when feasible: `new()` and `update()`
- **No New Patterns**: Don't introduce new abstractions unless you delete two
- **Benchmark-Driven**: If performance matters, measure it (don't guess)
- **Time-Boxed**: Propose what you could build in 30-60 minutes today

## Innovation Patterns from Practice

**Agent-to-Agent Handoff**: At major milestones, write handoff notes for next session (what worked, what didn't, decisions made, next steps)

**Test-Driven Infrastructure**: Build scripts and tools are testable too (validate outputs, exit codes, file generation)

**Blog Posts as Artifacts**: Reflective posts become documentation source material (first-person AI perspective, concrete metrics, honest about failures)

**Code as Disposable**: When SPEC + tests are comprehensive, code becomes regenerable (durable artifacts: SPEC, tests, LEARNINGS)

## Remember

Your mandate is not to produce maximal code, but to produce maximal clarity with minimal code.

**Understand why these practices work:**
- Not workarounds for current limitations
- Optimal collaboration structure for human-AI development
- Built on economic invariants (generation is cheap, clarity is valuable)

**Execute these practices knowing:**
- Drafts are fuel; insights are the product
- Artifacts are disposable; clarity is durable
- Comprehensive generation enables focused simplification

**Skiwave-specific focus:**
- Procedural generation quality (noise coherence, FFT synthesis)
- Audio-visual synchronization (music drives terrain, motion modulates sound)
- Performance targets (60 FPS, <16ms frame budget)
- Flow mechanics (smooth motion → melodic world, lost rhythm → chaos)

Operate accordingly. Build the instrument. Surf the sound.
