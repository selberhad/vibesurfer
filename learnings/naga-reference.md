# Naga Reference - Shader Translation for Rust Projects

**Purpose**: Comprehensive reference for using Naga in any Rust project with shaders

**Audience**: AI agents working with WGSL, SPIR-V, or cross-platform graphics

**Scope**: Architecture, validation, translation workflows, error handling, platform gotchas

---

## Quick Start

### Via wgpu (Typical Usage)

```rust
// wgpu handles Naga automatically
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("my_shader"),
    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
});

// wgpu internally: parse → validate → translate → compile
```

### Standalone (Shader Tooling)

```rust
use naga::front::wgsl;
use naga::valid::{Validator, ValidationFlags, Capabilities};
use naga::back::msl;

// Parse WGSL
let module = wgsl::parse_str(wgsl_source)?;

// Validate
let module_info = Validator::new(
    ValidationFlags::all(),
    Capabilities::all(),
).validate(&module)?;

// Translate to Metal Shading Language
let mut msl_source = String::new();
msl::Writer::new(&mut msl_source).write(
    &module,
    &module_info,
    &msl::Options::default(),
    &msl::PipelineOptions::default(),
)?;
```

---

## What is Naga?

**Naga** = Shader translation library for wgpu ecosystem

**Mental model**: Compiler middle-end for shaders
- **Frontend**: Parse source (WGSL, SPIR-V, GLSL) → IR
- **Validator**: Type-check and verify IR correctness
- **Backend**: IR → platform shader (SPIR-V, MSL, HLSL, GLSL)

**Not**: GPU driver or runtime graphics API (that's wgpu)
**Yes**: Translation and validation layer between source and GPU

---

## Core Concepts

### Naga IR (Intermediate Representation)

**Central type**: `naga::Module`
- Rust data structure representing entire shader program
- Contains: types, constants, globals, functions, entry points
- Language-agnostic (same for WGSL, SPIR-V, GLSL inputs)

**Design**: Arena-based allocation
- `Arena<T>` - Container for IR nodes (types, expressions, statements)
- `Handle<T>` - Typed reference into arena (like a safe index)
- Enables fast lookups and transformations

### Three-Stage Pipeline

```
Source (WGSL/SPIR-V/GLSL)
    ↓
naga::front::* → naga::Module (IR)
    ↓
naga::valid::Validator → naga::valid::ModuleInfo
    ↓
naga::back::* → Platform Shader (SPIR-V/MSL/HLSL)
```

**Validation is mandatory**:
- Backends require `ModuleInfo` from validation
- Can't skip validation (will panic or produce incorrect output)
- Cost is negligible (< 1ms for typical shaders)

---

## Frontends (Input Parsers)

| Frontend | Status | Feature Flag | Input Format | Notes |
|----------|--------|--------------|--------------|-------|
| WGSL | ✅ Primary | `wgsl-in` | Text | Fully validated, wgpu's primary language |
| SPIR-V | ✅ Primary | `spv-in` | Binary | Vulkan standard, mature |
| GLSL | ⚠️ Secondary | `glsl-in` | Text | GLSL 440+ / Vulkan semantics only |

### WGSL Frontend

```rust
use naga::front::wgsl;

let module = wgsl::parse_str(wgsl_source)?;
// or
let module = wgsl::parse_str(include_str!("shader.wgsl"))?;
```

**Errors**: `wgsl::ParseError`
- Syntax errors (missing semicolon, invalid token)
- Semantic errors (type mismatch, undefined variable)
- Includes source spans for error reporting

**Recommendation**: Use WGSL for new projects (wgpu's native language)

### SPIR-V Frontend

```rust
use naga::front::spv;

let spv_binary: &[u32] = /* SPIR-V binary words */;
let options = spv::Options::default();
let module = spv::parse_u32_slice(spv_binary, &options)?;
```

**Use case**: Loading pre-compiled Vulkan shaders

### GLSL Frontend

```rust
use naga::front::glsl;

let mut parser = glsl::Frontend::default();
let options = glsl::Options::from(naga::ShaderStage::Fragment);
let module = parser.parse(&options, glsl_source)?;
```

**Limitation**: Vulkan GLSL only (not legacy OpenGL GLSL)

---

## Validation

### Core API

```rust
use naga::valid::{Validator, ValidationFlags, Capabilities};

let mut validator = Validator::new(
    ValidationFlags::all(),  // Which checks to perform
    Capabilities::all(),     // Which GPU features to allow
);

let module_info = validator.validate(&module)?;
```

**Returns**:
- `Ok(ModuleInfo)` - Required by backends, contains type info and metadata
- `Err(WithSpan<ValidationError>)` - Error with source location

### What Validation Checks

**Type correctness**:
- Operations match operand types (vec3 + vec3, not vec3 + float)
- Function signatures correct
- Return values match declared types

**Resource bindings**:
- No duplicate `@group(X) @binding(Y)`
- Texture/buffer types match usage
- Location attributes valid for shader stage

**Control flow**:
- All code paths return (for non-void functions)
- No unreachable code
- Uniform control flow where required

**Platform capabilities**:
- Features used are in allowed `Capabilities`
- Helps catch platform-incompatible code early

**Memory layouts**:
- Structs meet host-shareable alignment rules (for uniform/storage buffers)
- Array strides correct

### Validation Flags

**ValidationFlags** (bitflags):
- `EXPRESSIONS` - Expression type checking
- `BLOCKS` - Statement and block validation
- `CONTROL_FLOW_UNIFORMITY` - Uniform control flow requirements
- `STRUCT_LAYOUTS` - Host-shareable struct alignment
- `CONSTANTS` - Constant expression validation
- `BINDINGS` - Resource binding attributes

**Typical usage**: `ValidationFlags::all()` (enable all checks)

### Capabilities

**Controls allowed GPU features**:
- `Capabilities::all()` - Allow everything Naga supports (permissive)
- `Capabilities::default()` - Conservative baseline (cross-platform safe)
- Platform-specific capabilities - Match target GPU (Metal, Vulkan, DX)

**Best practice**: Validate with target platform capabilities
```rust
// For Metal (macOS)
let metal_caps = Capabilities::default();  // Conservative Metal subset
let mut validator = Validator::new(ValidationFlags::all(), metal_caps);
```

### Error Handling

```rust
match validator.validate(&module) {
    Ok(module_info) => { /* proceed to backend */ }
    Err(error) => {
        eprintln!("Validation error: {}", error);
        // error includes source spans for debugging
        for (span, context) in error.spans() {
            eprintln!("  at {:?}: {}", span, context);
        }
    }
}
```

**Error types** (hierarchical):
- `ValidationError::Type` - Type errors
- `ValidationError::Function` - Function body errors
- `ValidationError::EntryPoint` - Shader entry point signature errors
- `ValidationError::GlobalVariable` - Global resource errors
- `ValidationError::Constant` - Const evaluation errors

### Common Validation Errors

**"Type X cannot be used in uniform buffer"**:
- Fix: Adjust struct layout, add padding, or use storage buffer

**"Entry point function must return void"**:
- Fix: Use output parameters or builtin outputs, not return value

**"Expression X is not constant"**:
- Fix: Use `const` or `override` declarations

**"Binding X conflicts with binding Y"**:
- Fix: Assign unique `@group` and `@binding` indices

**"Capability X not supported"**:
- Fix: Enable capability in validator or rewrite shader

---

## Backends (Output Writers)

| Backend | Status | Feature Flag | Output | Platform |
|---------|--------|--------------|--------|----------|
| SPIR-V | ✅ Primary | `spv-out` | Binary | Vulkan |
| MSL | ✅ Primary | `msl-out` | Text | Metal (macOS, iOS) |
| HLSL | ✅ Primary | `hlsl-out` | Text | DirectX 11+, DX12 |
| GLSL | ⚠️ Secondary | `glsl-out` | Text | OpenGL 3.3+, GLES 3.0+ |
| WGSL | ⚠️ Secondary | `wgsl-out` | Text | Debugging, round-trip |
| DOT | ⚠️ Diagnostic | `dot-out` | GraphViz | Visualization |

### SPIR-V Backend

**Target**: Vulkan, OpenCL
**Output**: Binary (Vec<u32>)

```rust
use naga::back::spv;

let mut spv_binary = Vec::new();
spv::write_vec(
    &module,
    &module_info,
    &spv::Options {
        lang_version: (1, 3),  // SPIR-V 1.3 (Vulkan 1.1+)
        flags: spv::WriterFlags::DEBUG,  // Include debug info
        ..Default::default()
    },
    None,  // PipelineOptions (optional)
    &mut spv_binary,
)?;

// spv_binary is Vec<u32> ready for vkCreateShaderModule()
```

**Key options**:
- `lang_version: (u8, u8)` - SPIR-V version (default: `(1, 3)`)
- `flags: WriterFlags` - Debug info, coordinate adjustment
- `bounds_check_policies` - OOB access handling

**Gotcha**: SPIR-V is binary (use `spirv-dis` to disassemble for inspection)

### MSL Backend

**Target**: Metal (macOS, iOS, tvOS)
**Output**: Text (Metal Shading Language source)

```rust
use naga::back::msl;

let mut msl_source = String::new();
msl::Writer::new(&mut msl_source).write(
    &module,
    &module_info,
    &msl::Options {
        lang_version: (2, 4),  // MSL 2.4 (macOS 11+, iOS 14+)
        zero_initialize_workgroup_memory: true,
        ..Default::default()
    },
    &msl::PipelineOptions::default(),
)?;

// msl_source is String ready for MTLLibrary compilation
```

**Key options**:
- `lang_version: (u8, u8)` - MSL version (default: `(2, 0)`)
- `per_entry_point_map` - Binding mappings (Metal has flat binding model)
- `zero_initialize_workgroup_memory: bool` - Polyfill workgroup zeroing

**Gotcha**: Metal doesn't have descriptor sets (WGSL `@group`/`@binding` must map to flat indices)

### HLSL Backend

**Target**: DirectX (D3D11, D3D12)
**Output**: Text (HLSL source)

```rust
use naga::back::hlsl;

let mut hlsl_source = String::new();
hlsl::Writer::new(&mut hlsl_source, &hlsl::Options {
    shader_model: hlsl::ShaderModel::V5_0,  // D3D11
    ..Default::default()
}).write(&module, &module_info)?;
```

**Key options**:
- `shader_model` - SM 5.0 (D3D11), SM 5.1/6.0+ (D3D12)
- `binding_map` - Map WGSL groups → HLSL registers (t#, s#, u#, b#)

### GLSL Backend

**Target**: OpenGL 3.3+, OpenGL ES 3.0+
**Output**: Text (GLSL source)

```rust
use naga::back::glsl;

let mut glsl_source = String::new();
glsl::Writer::new(
    &mut glsl_source,
    &module,
    &module_info,
    &glsl::Options::default(),
    &glsl::PipelineOptions {
        shader_stage: naga::ShaderStage::Fragment,
        entry_point: "main_fs".to_string(),
        multiview: None,
    },
    naga::proc::BoundsCheckPolicies::default(),
)?.write()?;
```

**Gotcha**: Targets Vulkan GLSL (GLSL 440+), not legacy OpenGL GLSL

---

## Complete Workflows

### WGSL → SPIR-V (Vulkan)

```rust
// Parse
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Validate
let module_info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
).validate(&module)?;

// Translate
let mut spv_binary = Vec::new();
naga::back::spv::write_vec(
    &module,
    &module_info,
    &naga::back::spv::Options::default(),
    None,
    &mut spv_binary,
)?;

// Use spv_binary with Vulkan API
```

### WGSL → MSL (Metal)

```rust
// Parse
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Validate (with Metal capabilities)
let metal_caps = naga::valid::Capabilities::default();
let module_info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    metal_caps,
).validate(&module)?;

// Translate
let mut msl_source = String::new();
naga::back::msl::Writer::new(&mut msl_source).write(
    &module,
    &module_info,
    &naga::back::msl::Options {
        lang_version: (2, 4),
        ..Default::default()
    },
    &naga::back::msl::PipelineOptions::default(),
)?;

// Compile with Metal: xcrun metal -c shader.metal
```

### Shader Validation (CI/Linting)

```rust
fn validate_shader(path: &Path) -> Result<(), String> {
    let source = std::fs::read_to_string(path)?;
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| format!("Validation error: {}", e))?;

    Ok(())
}
```

### Multi-Platform Translation

```rust
// Parse and validate once
let module = naga::front::wgsl::parse_str(wgsl_source)?;
let module_info = validator.validate(&module)?;

// Translate to multiple targets
let mut spv_binary = Vec::new();
naga::back::spv::write_vec(&module, &module_info, &spv::Options::default(), None, &mut spv_binary)?;

let mut msl_source = String::new();
naga::back::msl::Writer::new(&mut msl_source)
    .write(&module, &module_info, &msl::Options::default(), &msl::PipelineOptions::default())?;

let mut hlsl_source = String::new();
naga::back::hlsl::Writer::new(&mut hlsl_source, &hlsl::Options::default())
    .write(&module, &module_info)?;
```

---

## Platform Differences and Gotchas

### Coordinate Systems

**Clip space Y**:
- Vulkan: Y-down (inverted from WGSL/Naga IR)
- Metal/DX: Y-up (matches WGSL/Naga IR)
- Use `WriterFlags::ADJUST_COORDINATE_SPACE` for Vulkan if needed

**Texture coordinates**:
- Vulkan: Y-down
- Metal: Y-up
- Handle in shader or vertex data

**Depth range**:
- Vulkan: [0, 1]
- OpenGL: [-1, 1]

### Binding Models

**WGSL**: Two-level hierarchy
```wgsl
@group(0) @binding(1) var my_texture: texture_2d<f32>;
```

**SPIR-V (Vulkan)**: Descriptor sets + bindings
- Group → Descriptor set
- Binding → Binding index within set

**MSL (Metal)**: Flat indices
- Must map WGSL groups/bindings → Metal argument indices
- wgpu handles automatically via `per_entry_point_map`

**HLSL (DirectX)**: Register spaces
- `t#` - Textures/buffers (SRV)
- `s#` - Samplers
- `u#` - UAVs (unordered access)
- `b#` - Constant buffers

### Feature Availability

**f64 (double precision)**:
- Desktop Vulkan: ✅
- Metal: ❌ (not supported)
- Mobile: ❌ (most GPUs)

**Int64 atomics**:
- Vulkan: ✅
- Metal: ❌ (Naga backend doesn't support)
- DX12: ✅

**Subgroup operations**:
- Vulkan: ✅ (full support)
- Metal: ⚠️ (partial, SIMD-group ops)
- DX12: ✅ (wave ops)

**Ray tracing**:
- Vulkan: ✅ (SPIR-V backend)
- Metal: ❌ (use Metal's native ray tracing, not via Naga)
- DX12: ✅ (HLSL backend)

### Workgroup Memory Initialization

**Vulkan**: Zero-initialized by default
**Metal**: Not guaranteed zero
**DX**: Zero-initialized by default

**Solution**: Use `zero_initialize_workgroup_memory: true` in MSL options

---

## Error Debugging

### Distinguish Error Sources

**Naga parse error** (`wgsl::ParseError`):
- Syntax error in source code
- Happens during `parse_str()`

**Naga validation error** (`ValidationError`):
- IR correctness issue (types, bindings, control flow)
- Happens during `validator.validate()`

**Naga backend error** (backend-specific error types):
- Platform compatibility issue
- Happens during `Writer::write()`
- Example: Using f64 when translating to MSL

**Driver error** (platform-specific strings):
- GPU driver rejected shader
- Happens during driver compilation (vkCreateShaderModule, MTLLibrary creation)
- Not caught by Naga

### Using Spans for Error Location

```rust
match validator.validate(&module) {
    Err(error) => {
        eprintln!("Validation error: {}", error);

        // Map spans back to source lines
        for (span, context) in error.spans() {
            // span.start, span.end are byte offsets in source
            eprintln!("  at byte {}..{}: {}", span.start, span.end, context);
        }
    }
    Ok(_) => { /* ... */ }
}
```

**Tools for better error messages**:
- `codespan-reporting` crate - Pretty-print errors with source context
- `naga-cli` - Validate from command line with formatted errors

### Debugging Translation

**Use DOT backend for visualization**:
```rust
let mut dot_output = String::new();
naga::back::dot::write(&module, &mut dot_output)?;
std::fs::write("shader.dot", dot_output)?;
// dot -Tpng shader.dot -o shader.png
```

**Compare Naga output to reference**:
- Write shader by hand in target language
- Compare Naga translation output
- Identify differences or issues

**Test with naga-cli**:
```bash
naga shader.wgsl shader.msl  # Translate WGSL → MSL
naga shader.wgsl --validate  # Validate only
```

---

## Performance Characteristics

### Parse + Validate + Translate Cost

**Typical shaders** (100-500 lines):
- Parse: < 1ms
- Validate: < 1ms
- Translate: < 1ms
- **Total: ~1-3ms**

**Large shaders** (1000+ lines):
- Parse: ~5ms
- Validate: ~5ms
- Translate: ~5ms
- **Total: ~10-20ms**

**Recommendation**: Validation at runtime is acceptable (cost is negligible)

### When to Pre-Compile

**Build-time validation**:
- Use `naga-cli` or build script to validate shaders offline
- Catch errors during build, not runtime
- Useful for CI/CD

**Pre-compile to SPIR-V**:
- SPIR-V is binary, stable across Naga versions
- Cache SPIR-V for faster startup (skip parse + validate + translate)
- Ship SPIR-V instead of WGSL if code size matters

**Pre-compile to MSL/HLSL**:
- Less stable (Naga output may change across versions)
- Only cache if version-locked to specific Naga release

---

## Tools and Utilities

### naga-cli

**Installation**:
```bash
cargo install naga-cli
```

**Usage**:
```bash
# Validate only
naga shader.wgsl

# Translate WGSL → MSL
naga shader.wgsl shader.metal

# Translate SPIR-V → WGSL (round-trip)
naga shader.spv shader.wgsl

# Dump IR to text
naga shader.wgsl shader.txt
```

**Integration**:
- Add to CI pipeline (validate shaders on commit)
- Use in build scripts (pre-compile to target formats)
- Test translation offline (debug output)

### Platform Shader Validators

**Validate translated shaders**:
```bash
# SPIR-V (requires SPIRV-Tools)
cargo xtask validate spv

# MSL (requires Xcode command-line tools)
cargo xtask validate msl
xcrun metal -c shader.metal  # Direct Metal compile

# GLSL (requires GLSLang)
cargo xtask validate glsl

# HLSL (requires DXC or FXC)
cargo xtask validate hlsl dxc
```

**Best practice**: Test translated output on target platform (driver quirks exist)

---

## Integration with wgpu

### wgpu Uses Naga Internally

```rust
// This single call triggers Naga pipeline:
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("my_shader"),
    source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
});

// Internally:
// 1. naga::front::wgsl::parse_str(wgsl_source)
// 2. naga::valid::Validator::new(...).validate(&module)
// 3. Backend translation (SPIR-V for Vulkan, MSL for Metal, etc.)
// 4. Driver compilation (vkCreateShaderModule, MTLLibrary, etc.)
```

**Error handling**:
```rust
match device.create_shader_module(desc) {
    Ok(shader_module) => { /* ... */ }
    Err(e) => {
        eprintln!("Shader creation failed: {}", e);
        // e.source() may include Naga validation errors
    }
}
```

### When to Use Naga Directly vs wgpu

**Use wgpu** (typical apps):
- Normal graphics application development
- wgpu handles Naga for you
- Simpler API, integrated error reporting

**Use Naga directly**:
- Shader tooling (linters, formatters, analyzers)
- Build-time validation (CI/CD)
- Offline translation (asset pipeline)
- Custom workflows (compute-only, non-wgpu pipelines)
- Debugging (inspect IR, test translation)

### Feature Flags

**wgpu features** (enable Naga frontends):
- `wgsl` - WGSL input (enabled by default)
- `spirv` - SPIR-V input
- `glsl` - GLSL input
- `naga-ir` - Direct Naga IR input

**Naga features** (when using standalone):
- `wgsl-in`, `spv-in`, `glsl-in` - Frontends
- `spv-out`, `msl-out`, `hlsl-out`, `glsl-out`, `wgsl-out`, `dot-out` - Backends

---

## Common Patterns

### Pattern: Validate Shader at Build Time

```rust
// build.rs
fn main() {
    let wgsl_path = "src/shaders/compute.wgsl";
    let wgsl_source = std::fs::read_to_string(wgsl_path).unwrap();

    let module = naga::front::wgsl::parse_str(&wgsl_source)
        .expect("Failed to parse shader");

    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("Shader validation failed");

    println!("cargo:rerun-if-changed={}", wgsl_path);
}
```

### Pattern: Cache Translated Shaders

```rust
// Generate SPIR-V at build time, embed in binary
fn main() {
    let wgsl_source = std::fs::read_to_string("shader.wgsl").unwrap();
    let module = naga::front::wgsl::parse_str(&wgsl_source).unwrap();
    let module_info = validator.validate(&module).unwrap();

    let mut spv_binary = Vec::new();
    naga::back::spv::write_vec(&module, &module_info, &spv::Options::default(), None, &mut spv_binary).unwrap();

    // Write to file, embed via include_bytes!
    std::fs::write("shader.spv", bytemuck::cast_slice(&spv_binary)).unwrap();
}

// In runtime code:
let spv_bytes = include_bytes!("shader.spv");
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    source: wgpu::ShaderSource::SpirV(bytemuck::cast_slice(spv_bytes).into()),
    ..Default::default()
});
```

### Pattern: Multi-Backend Shader Export

```rust
// Export shader to all platforms
fn export_shader(wgsl_source: &str, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let module = naga::front::wgsl::parse_str(wgsl_source)?;
    let module_info = validator.validate(&module)?;

    // SPIR-V (Vulkan)
    let mut spv_binary = Vec::new();
    naga::back::spv::write_vec(&module, &module_info, &Default::default(), None, &mut spv_binary)?;
    std::fs::write(output_dir.join("shader.spv"), bytemuck::cast_slice(&spv_binary))?;

    // MSL (Metal)
    let mut msl_source = String::new();
    naga::back::msl::Writer::new(&mut msl_source)
        .write(&module, &module_info, &Default::default(), &Default::default())?;
    std::fs::write(output_dir.join("shader.metal"), msl_source)?;

    // HLSL (DirectX)
    let mut hlsl_source = String::new();
    naga::back::hlsl::Writer::new(&mut hlsl_source, &Default::default())
        .write(&module, &module_info)?;
    std::fs::write(output_dir.join("shader.hlsl"), hlsl_source)?;

    Ok(())
}
```

---

## Key Takeaways

**Architecture**: Frontend → IR → Validator → Backend
- Parse source to `Module` (IR)
- Validate to get `ModuleInfo` (required by backends)
- Translate IR to platform shader via backend

**Validation is mandatory**:
- Backends require `ModuleInfo`
- Cost is negligible (< 1ms)
- Catches errors before GPU execution

**Platform differences matter**:
- Coordinate systems differ (Y-up vs Y-down)
- Binding models differ (descriptor sets vs flat indices vs registers)
- Feature availability differs (f64, int64 atomics, subgroups)
- Validate with platform-specific capabilities when possible

**wgpu vs standalone**:
- wgpu uses Naga internally (automatic)
- Use Naga standalone for tooling, build-time validation, custom workflows

**Debugging**:
- Validation errors include source spans
- Use DOT backend for IR visualization
- Use naga-cli for offline testing
- Test translated output on target platform

---

## References

**Documentation**:
- [Naga docs.rs](https://docs.rs/naga/latest/naga/)
- [wgpu docs.rs](https://docs.rs/wgpu/latest/wgpu/)
- [WGSL Specification](https://www.w3.org/TR/WGSL/)
- [SPIR-V Specification](https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html)

**Related learnings** (in this repo):
- `learnings/naga-architecture.md` - Deep dive on Naga structure
- `learnings/naga-validation.md` - Validation details and error handling
- `learnings/naga-translation.md` - Backend translation and platform quirks

**Tools**:
- `naga-cli` - Command-line shader tool
- `spirv-dis` / `spirv-val` - SPIR-V disassembly and validation (SPIRV-Tools)
- `xcrun metal` - Metal shader compiler (macOS)
- `dxc` / `fxc` - HLSL compilers (Windows)

**Source**:
- [Naga GitHub](https://github.com/gfx-rs/wgpu/tree/trunk/naga)
- [wgpu GitHub](https://github.com/gfx-rs/wgpu)

---

**Created**: October 2025
**Last updated**: October 2025
**Status**: Research phase complete (Priorities 0-2 documented)
**Next**: Discovery phase (validate learnings with toy implementation)
