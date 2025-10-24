# Naga Architecture - Foundational Understanding

**Purpose**: Core mental model of Naga's role, architecture, and when to use it

**Audience**: AI agents working with shaders in Rust projects

**Key insight**: Naga is shader IR + translation. Parser → validated IR → backend writer. Standalone usable, wgpu-integrated by default.

---

## What is Naga?

**Naga** = "Shader translation library for the needs of wgpu"

**Core purpose**: Translate shader source code between languages (WGSL ↔ SPIR-V ↔ GLSL ↔ MSL ↔ HLSL)

**Mental model**: Compiler middle-end for shaders
- **Frontend**: Parse source → IR (Intermediate Representation)
- **Validator**: Check IR correctness (types, bindings, capabilities)
- **Backend**: IR → target language output

**Not**: A runtime graphics API (that's wgpu)
**Not**: A driver or GPU abstraction (that's wgpu-hal)
**Yes**: Translation and validation layer sitting between source code and drivers

---

## Architecture: Three-Stage Pipeline

```
Source Code (WGSL/SPIR-V/GLSL)
    ↓
[Frontend] Parse → naga::Module (IR)
    ↓
[Validator] Check correctness → naga::valid::ModuleInfo
    ↓
[Backend] Write → Target Code (SPIR-V/MSL/HLSL/GLSL/WGSL)
```

### Stage 1: Frontend (naga::front)

**Purpose**: Parse shader source into Naga IR

**Supported frontends**:
- `wgsl` - WGSL parser (✅ primary, fully validated)
- `spv` - SPIR-V binary parser (✅ primary)
- `glsl` - GLSL parser (⚠️ secondary, GLSL 440+ / Vulkan semantics only)

**Key type**: `naga::Module`
- IR structure representing entire shader module
- Contains: types, constants, global variables, functions, entry points
- Language-agnostic (same IR for all frontends)

**Example**: Parsing WGSL
```rust
let wgsl_source = "@fragment fn main_fs() -> @location(0) vec4<f32> { ... }";
let module: naga::Module = naga::front::wgsl::parse_str(wgsl_source)?;
```

### Stage 2: Validator (naga::valid)

**Purpose**: Verify IR correctness before translation

**Key types**:
- `naga::valid::Validator` - Configurable validation engine
- `naga::valid::ModuleInfo` - Metadata from validation (required by backends)
- `naga::valid::ValidationFlags` - What checks to perform
- `naga::valid::Capabilities` - What GPU features to allow

**What validation checks**:
- Type correctness (operations match operand types)
- Binding correctness (textures/uniforms match expected layout)
- Capability support (does target GPU support requested features?)
- Entry point validity (shader stage signatures correct)
- Control flow (no invalid branches, returns)

**Example**: Validating a module
```rust
let module_info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
)
.validate(&module)?;
```

**Critical**: Backends require `ModuleInfo` from validation. Always validate before writing.

### Stage 3: Backend (naga::back)

**Purpose**: Translate validated IR to target shader language

**Supported backends**:
- `spv` - SPIR-V binary (✅ primary, for Vulkan)
- `msl` - Metal Shading Language (✅ primary, for Metal/macOS/iOS)
- `hlsl` - HLSL Shader Model 5.0+ (✅ primary, for DirectX 11+)
- `glsl` - GLSL 330+ / GLSL ES 300+ (⚠️ secondary)
- `wgsl` - WGSL output (⚠️ secondary, for debugging/roundtrip)
- `dot` - GraphViz DOT (⚠️ visualization, not executable shader)

**Key pattern**: Writer types (e.g., `glsl::Writer`, `spv::Writer`, `msl::Writer`)
- Takes `&Module`, `&ModuleInfo`, options
- Writes to string or binary output
- Backend-specific options (entry point, shader stage, platform quirks)

**Example**: Translating to GLSL
```rust
use naga::back::glsl;
let mut glsl_source = String::new();
glsl::Writer::new(
    &mut glsl_source,
    &module,
    &module_info,
    &glsl::Options::default(),
    &glsl::PipelineOptions {
        entry_point: "main_fs".into(),
        shader_stage: naga::ShaderStage::Fragment,
        multiview: None,
    },
    naga::proc::BoundsCheckPolicies::default(),
)?.write()?;
```

---

## Naga's Role in the wgpu Ecosystem

**wgpu stack**:
```
Application (vibesurfer)
    ↓
wgpu (high-level API: Device, Queue, ShaderModule)
    ↓
wgpu-core (backend-agnostic implementation)
    ↓
Naga (shader translation/validation) ← YOU ARE HERE
    ↓
wgpu-hal (hardware abstraction: Vulkan, Metal, DX12, etc.)
    ↓
Platform Drivers (Vulkan/Metal/DX12 drivers)
```

**wgpu uses Naga internally**:
- When you call `device.create_shader_module(...)`, wgpu invokes Naga
- WGSL source → Naga frontend → validate → translate to platform shader (SPIR-V for Vulkan, MSL for Metal)
- You don't directly call Naga when using wgpu APIs

**When to use Naga directly**:
- **Shader tooling**: Build linters, formatters, analyzers
- **Pre-compilation**: Validate shaders at build time (not runtime)
- **Translation pipelines**: Convert shader assets offline
- **Debugging**: Inspect IR, test validation, understand errors
- **Custom workflows**: Non-wgpu shader pipelines (e.g., compute-only, offline tools)

**When to use via wgpu**:
- **Normal application development**: wgpu handles Naga for you
- **Runtime shader loading**: wgpu's `ShaderModule` API is simpler
- **Integrated validation**: wgpu reports Naga errors with helpful context

**Feature flags** (when using Naga directly):
- `wgsl-in` - Enable WGSL frontend
- `spv-in` - Enable SPIR-V frontend
- `glsl-in` - Enable GLSL frontend
- `spv-out` - Enable SPIR-V backend
- `msl-out` - Enable MSL backend
- `hlsl-out` - Enable HLSL backend
- `glsl-out` - Enable GLSL backend
- `wgsl-out` - Enable WGSL backend
- `dot-out` - Enable GraphViz DOT backend

---

## Naga IR vs SPIR-V

**SPIR-V**: Binary intermediate representation for Vulkan/OpenCL
- Standard format (Khronos spec)
- Platform-neutral, driver-ready
- Binary encoding (compact, opaque)

**Naga IR**: Rust data structures for shader representation
- Rust-native (uses Rust types, `Arena<T>`, `Handle<T>`)
- Higher-level than SPIR-V (closer to source languages)
- Designed for analysis and transformation

**Relationship**:
- Naga can parse SPIR-V → Naga IR (`spv-in`)
- Naga can emit Naga IR → SPIR-V (`spv-out`)
- Naga IR is NOT a 1:1 mapping to SPIR-V (higher abstraction)
- Translation is lossy in edge cases (see learnings/naga-translation.md)

**Why Naga IR exists**:
- SPIR-V is hard to analyze directly (binary format, low-level constructs)
- Naga IR is Rust-friendly (type-safe, inspectable, transformable)
- Supports multiple source languages (WGSL, GLSL) without SPIR-V round-trip
- Validation and optimization on IR easier than on SPIR-V

---

## Common API Patterns

### Pattern 1: Parse + Validate + Translate (full pipeline)

```rust
// Parse source
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Validate
let module_info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
).validate(&module)?;

// Translate to target
let mut output = String::new();
naga::back::msl::Writer::new(&mut output)
    .write(&module, &module_info, &msl::Options::default())?;
```

### Pattern 2: Standalone Validation (CI/linting)

```rust
let module = naga::front::wgsl::parse_str(wgsl_source)?;
match naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
).validate(&module) {
    Ok(_) => println!("Shader valid"),
    Err(e) => eprintln!("Validation error: {:?}", e),
}
```

### Pattern 3: IR Inspection (debugging)

```rust
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Inspect module structure
println!("Types: {:?}", module.types);
println!("Functions: {:?}", module.functions);
println!("Entry points: {:?}", module.entry_points);

// Or dump to GraphViz
let mut dot_output = String::new();
naga::back::dot::write(&module, &mut dot_output)?;
// dot_output can be rendered with graphviz tools
```

---

## Key Types Reference

**Core IR**:
- `naga::Module` - Complete shader program (types, functions, globals, entry points)
- `naga::Handle<T>` - Type-safe reference into an arena (like a typed index)
- `naga::Arena<T>` - Container for IR nodes (types, expressions, statements)

**Validation**:
- `naga::valid::Validator` - Configurable validation engine
- `naga::valid::ModuleInfo` - Metadata produced by validation
- `naga::valid::ValidationFlags` - Which checks to enable
- `naga::valid::Capabilities` - GPU feature support requirements

**Frontend types** (input parsing):
- `naga::front::wgsl::ParseError` - WGSL syntax/semantic errors
- `naga::front::spv::Error` - SPIR-V parsing errors
- `naga::front::glsl::ParseError` - GLSL parsing errors

**Backend types** (output writing):
- `naga::back::spv::Writer` - SPIR-V binary writer
- `naga::back::msl::Writer` - MSL text writer
- `naga::back::glsl::Writer` - GLSL text writer
- `naga::back::hlsl::Writer` - HLSL text writer

---

## Constraints

**Validation is not optional**:
- Backends require `ModuleInfo` from `Validator.validate()`
- Skipping validation = runtime panic or incorrect output
- Validation cost is negligible compared to GPU execution

**Frontend coverage varies**:
- WGSL: Fully supported, actively maintained (wgpu's primary language)
- SPIR-V: Mature, stable (Vulkan standard)
- GLSL: Partial (Vulkan semantics only, no legacy OpenGL)

**Backend fidelity varies**:
- SPIR-V, MSL, HLSL: Primary targets, well-tested
- GLSL, WGSL: Secondary (GLSL for legacy, WGSL for debugging)
- Translation is not always 1:1 (see learnings/naga-translation.md)

**Feature flags required**:
- Naga uses cargo features to gate frontends/backends
- Reduces compile time + binary size when not all languages needed
- Must enable features matching your workflow (e.g., `wgsl-in`, `spv-out`)

---

## Gotchas

**Validator must match target capabilities**:
- If validating for Metal (macOS), use Metal-compatible capabilities
- Validating with `Capabilities::all()` may pass shaders that fail on target
- Check platform constraints before validation

**ModuleInfo is tied to a Module**:
- Don't reuse `ModuleInfo` across different modules
- Always validate before backend write

**Error messages reference source spans**:
- Naga tracks source locations (`Span`, `SourceLocation`)
- Use `WithSpan` wrapper for human-readable error reporting
- Point errors back to source code lines (useful for tooling)

**IR is not stable across versions**:
- Naga IR is internal (no stability guarantees)
- If serializing IR, version-lock Naga dependency
- Prefer source formats (WGSL/SPIR-V) for long-term storage

---

## Open Questions (for Discovery Phase)

**Q1**: What validation errors are most common when writing WGSL?
- Need practical testing with vibesurfer shaders (compute + fragment)
- What mistakes trigger validation failures?

**Q2**: How do Naga errors surface in wgpu?
- When wgpu calls Naga internally, how are errors reported?
- Can we distinguish Naga validation errors from driver errors?

**Q3**: What's the performance cost of validation?
- Is it cheap enough to validate every shader at runtime?
- Or should we pre-validate at build time?

**Q4**: What gets lost in WGSL → SPIR-V → MSL round-trip?
- Translation fidelity testing needed
- Platform-specific quirks?

**Q5**: How to use naga-cli for offline shader testing?
- Tool exists (`cargo install naga-cli`)
- Integration into build pipeline?

---

## References

**Primary documentation**:
- [Naga docs.rs](https://docs.rs/naga/latest/naga/) (cached: `.webcache/naga/docs-rs-main.html`)
- [Naga README](https://github.com/gfx-rs/wgpu/tree/trunk/naga) (cached: `.webcache/naga/naga-readme.md`)
- [wgpu docs.rs](https://docs.rs/wgpu/latest/wgpu/) (cached: `.webcache/naga/wgpu-main.html`)

**Supported languages**:
- Frontends: WGSL (primary), SPIR-V (primary), GLSL (secondary)
- Backends: SPIR-V (primary), MSL (primary), HLSL (primary), GLSL (secondary), WGSL (debug)

**Tools**:
- `naga-cli` - Command-line shader translation/validation tool
- `cargo xtask validate` - Test translated shaders on target platforms

---

**Created**: October 2025
**Last updated**: October 2025
**Status**: Priority 0 complete (foundational architecture documented)
