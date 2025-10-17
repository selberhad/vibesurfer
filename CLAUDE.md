# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Vibesurfer** - A fluid, retro-futuristic jet-surfing simulator where you glide, dive, and carve across an endless neon ocean. The surface behaves like living music: waves pulse to the beat, currents shimmer with color, and your motion becomes rhythm.

**Philosophy**: Artifacts are disposable, clarity is durable. Code can be rewritten, insights cannot. Generation is cheap, understanding is valuable.

**Navigation**: See `VISION.md` for project vision and architecture intent. See `FLOWFIELD.md` for the sound-surface feedback loop concept.

## Core Methodology

This project follows **Dialectic-Driven Development (DDD)** - a learning-driven workflow optimized for human-AI collaboration, orchestrated by **[Hegel](https://github.com/dialecticianai/hegel-cli)**.

> **Note**: Hegel must be installed and available in PATH. Check with `command -v hegel`. If not installed, clone hegel-cli and run `cargo build --release`, then add to PATH or use `./target/release/hegel`.

**Meta-mode**: `learning` (Research ↔ Discovery loop for greenfield exploration)

**Cycle**: Docs → Tests → Implementation → Learnings

**Long-term deliverable**: A playable, procedurally-generated surfing experience that captures pure flow. Documentation captures architectural insights and methodology learnings as we build.

### Using Hegel for Workflow Orchestration

**When to use hegel workflows**:
- Building toy experiments to validate unfamiliar techniques (Discovery mode)
- Researching external knowledge before implementation (Research mode)
- Complex features requiring structured planning and learning capture

**When to work directly**:
- Simple, straightforward tasks
- Bug fixes or small refactors
- User hasn't requested structured workflow

**Check status** before starting:
```bash
hegel status  # View current workflow and phase
```

**Start or transition workflows**:
```bash
hegel meta learning       # Declare learning meta-mode (first time only)
hegel start research      # External knowledge gathering
hegel start discovery     # Toy implementation and validation
```

**Advance through phases**:
```bash
hegel next     # Happy path (automatic claim inference)
hegel restart  # Return to beginning of cycle
hegel repeat   # Re-display current phase prompt
```

**How it works**:
- Each phase displays a prompt with embedded guidance (SPEC_WRITING, PLAN_WRITING, etc.)
- Follow the prompt to create the required artifact (SPEC.md, PLAN.md, LEARNINGS.md, etc.)
- State tracked in `.hegel/state.json`, transitions logged in `.hegel/states.jsonl`
- Workflow guides you through: SPEC → PLAN → CODE → LEARNINGS → README (Discovery mode)
- Run `hegel` without arguments for complete usage guide

## Guidance Vectors (from LEXICON)

**Context is king** - State matters more than features. What's visible determines what's possible.

**Artifacts are disposable, clarity is durable** - Code can be rewritten, insights cannot. Generation is cheap, understanding is valuable.

**Docs → Tests → Implementation → Learnings** - The DDD cycle. Specification before code. Reflection after execution.

**Infrastructure compounds** - Each tool enables new workflows. Each abstraction saves future tokens. Build once, reuse forever.

**Refactor early, not late** - 18x token overhead is immediate cost, not future debt. Structure for reading efficiency, not writing comfort.

**Write a script as soon as a pattern repeats** - If you're about to do it twice, stop and build the tool. Don't wait for pain.

**Remember you're not human** - Comprehensive is just complete. No cost to thoroughness. Human constraints don't apply.

**Domain language over implementation details** - Speak what it means, not how it works. `flow_tracker.update()` not `sigmoid_smoothness_calc()`.

## Recording Gameplay Videos

**Process**: Vibesurfer includes a built-in recording system that captures synchronized video frames and audio.

**Steps**:
1. **Clean previous recording** (if needed):
   ```bash
   rm -rf recording/frames/* recording/audio.wav recording/output.mp4
   ```

2. **Capture recording** (builds and runs with recording mode):
   ```bash
   cargo run --release -- --record 10  # 10 seconds
   ```
   - Captures 60fps PNG frames to `recording/frames/`
   - Captures synchronized audio to `recording/audio.wav`
   - Application exits automatically when recording completes

3. **Combine into video**:
   ```bash
   ./scripts/combine-recording.sh
   ```
   - Uses ffmpeg to merge frames + audio
   - Outputs `recording/output.mp4` (H.264, 60fps, AAC audio)
   - Displays file size and viewing command

4. **View result**:
   ```bash
   open recording/output.mp4
   ```

**Performance notes**:
- Use `--release` build for smooth 60fps capture
- Recording adds ~1ms overhead per frame (staging buffer copy)
- Typical output: ~2.5MB per second of video

**Camera options** (combine with `--record`):
```bash
cargo run --release -- --record 10 --camera-preset cinematic
cargo run --release -- --record 10 --camera-preset basic
cargo run --release -- --record 10 --camera-preset fixed --elevation 80
```

## Development Best Practices

**CRITICAL: Document Constraints**
- Modern Rust games have constraints (frame budgets, memory usage, GPU limits, audio latency)
- When hitting a constraint, document it in CONSTRAINTS.md with workaround
- "Why we can't do X" is as valuable as "how to do Y"
- Example: "Can't update 10k particles per frame (GPU bound). Workaround: LOD system with distance culling."

**CRITICAL: Test Assumptions Early**
- Procedural generation behavior is non-obvious (noise coherence, FFT synthesis, audio-visual sync)
- Build toy implementations to validate understanding BEFORE integrating into main game
- One toy per subsystem (wavefield_test, audio_synth_test, flow_tracker_test, etc.)
- **All testing must be automated** - use test harness, build tools, write benchmarks

**CRITICAL: Architecture Map Everything**
- Update CODE_MAP.md with architectural decisions
- Document module boundaries and data flow
- Track performance budgets (frame time, memory, audio buffer size)
- Note design patterns and why they were chosen

## Learning Documentation

Use hegel workflows (Research/Discovery modes) for structured exploration. Document findings in:
- `learnings/*.md` - Technical insights, constraints, patterns
- `learnings/.ddd/` - Meta-learnings from DDD cycles
- Update with real measurements after toy implementations

## Platform: macOS Apple Silicon (ARM64)

**CRITICAL**: Development machine is M1 MacBook Pro (arm64). Use Rust stable toolchain with arm64 target.

**Rust Toolchain**:
- Rust stable (latest)
- cargo for build management

**Graphics/Audio**:
- wgpu for cross-platform graphics (native + WebGPU)
- cpal for cross-platform audio
- Consider bevy or macroquad for game framework (TBD based on needs)

## Tooling

**Philosophy**: Pick the tool that allows the most concise, elegant solution with minimal dependencies.

**Engineering Discipline**:
- Never over-engineer. Try the simplest thing first.
- RTFM before building. Read docs, understand the problem, then act.
- Write a script when a pattern repeats - don't wait for pain.

**Stack**: Rust (core), shell scripts (automation), Python (analysis)

**Dependencies**: Minimize for core modules. Use established crates. Document non-obvious choices.

## Documentation Structure

### CODE_MAP.md Convention

**CRITICAL**: Update before any commit that changes structure or module boundaries.

Document:
- Module purpose and data flow
- WHY patterns were chosen (not just what they are)
- Performance characteristics

### Commit Guidelines
**Use conventional commit format for all commits:**
- **Format**: `type(scope): subject` with optional body/footer
- **Types**: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `perf`
- **Descriptive commits**: Include subsystem (e.g., "feat(wavefield): implement FFT synthesis")
- **History**: Keep linear history (prefer rebase; avoid merge commits)
- **Documentation updates**: Update affected CODE_MAP.md BEFORE committing

**Optional: Use Hegel's git guardrails**
```bash
# Wrap git commands for safety (if .hegel/guardrails.yaml configured)
hegel git add .
hegel git commit -m "feat(ocean): add FFT synthesis"
hegel git push
```

### Next Step Protocol

**Never just report what you did - always suggest what to do next:**
- After completing any task, propose the next logical action
- Don't say "done" or "ready for next step" - suggest a specific next move
- Identify next task from context or infer logical progression
- **Wait for explicit approval before proceeding**

**Format**: "Should I [specific action], or [alternative]?"

## Testing Philosophy

**For Rust Code:**
- Use Rust's built-in test framework (`#[test]`, `#[bench]`)
- Criterion.rs for detailed benchmarks with statistical analysis
- Property-based testing with proptest for procedural generation
- Integration tests for subsystem interactions

**Performance Validation:**
- Target: 60 FPS (16.67ms frame budget)
- Measure with criterion benchmarks and profiling tools

## Rust-Specific Guidelines

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

## Architecture Documentation

**CRITICAL**: Keep architectural documentation up-to-date
- **ARCHITECTURE.md**: System design, data flow, performance budget, key constraints
- **CODE_MAP.md**: Module-by-module navigation, entry points, integration points
- Update these BEFORE committing any structural changes

## Self-Audit Checklist (Before Proposing Changes)

- Tests pass (`cargo test`)
- Benchmarks meet targets (if performance-critical)
- Docs updated (CODE_MAP.md if structural, README.md if API changed)
- Commit message follows conventional format

## Simplification Heuristics

Apply before coding and before PR:

- **One-Module Rule**: Prefer single module to prove the concept
- **Two-Function Rule**: Two public entrypoints when feasible: `new()` and `update()`
- **No New Patterns**: Don't introduce new abstractions unless you delete two
- **Benchmark-Driven**: If performance matters, measure it (don't guess)
- **Time-Boxed**: Propose what you could build in 30-60 minutes today

## HANDOFF.md Protocol

**CRITICAL: Only update at END OF SESSION**

**Purpose**: Session-to-session continuity. Gitignored ephemeral file.

**At session start:**
- Read `HANDOFF.md` if exists
- **Delete after reading**: `rm HANDOFF.md` (force explicit handoff, prevent drift)

**At session end:**
- Write fresh `HANDOFF.md` (old already deleted)
- Include: Status, learnings, next action, key files
- **NO CODE WORK AFTER WRITING** - signals session end
- Only housekeeping: docs updates, commits
- **NEVER commit HANDOFF.md**

## Innovation Patterns from Practice

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

See ARCHITECTURE.md and VISION.md for the technical and aesthetic vision.

Operate accordingly. Build the instrument. Surf the sound.
