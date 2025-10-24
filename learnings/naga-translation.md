# Naga Translation - Cross-Platform Shader Compilation

**Purpose**: Understanding Naga's backend translation, platform-specific quirks, and output control

**Audience**: AI agents targeting multiple graphics backends (Vulkan, Metal, DirectX)

**Key insight**: Backends translate validated IR → platform shaders (SPIR-V, MSL, HLSL). Each backend has options, quirks, and fidelity differences. SPIR-V and MSL are primary; GLSL/WGSL are secondary.

---

## Translation Pipeline Overview

```
naga::Module (validated IR)
    +
naga::valid::ModuleInfo (from validation)
    ↓
[Backend Writer] (spv::Writer, msl::Writer, etc.)
    ↓
Platform Shader (SPIR-V binary, MSL text, HLSL text, etc.)
```

**Inputs** (required by all backends):
- `&Module` - The shader IR
- `&ModuleInfo` - Validation metadata (from `Validator::validate()`)
- Backend-specific options (language version, bindings, flags)

**Output**:
- Binary (SPIR-V) or text (MSL, HLSL, GLSL, WGSL)
- Backend may fail even if validation passed (platform-specific constraints)

---

## Backend Coverage

| Backend | Status | Feature Flag | Output | Primary Use |
|---------|--------|--------------|--------|-------------|
| SPIR-V | ✅ Primary | `spv-out` | Binary | Vulkan |
| MSL | ✅ Primary | `msl-out` | Text | Metal (macOS, iOS) |
| HLSL | ✅ Primary | `hlsl-out` | Text | DirectX 11+, DX12 |
| GLSL | ⚠️ Secondary | `glsl-out` | Text | OpenGL 3.3+, GLES 3.0+ |
| WGSL | ⚠️ Secondary | `wgsl-out` | Text | Debugging, round-trip testing |
| DOT | ⚠️ Diagnostic | `dot-out` | GraphViz | Visualization (not executable) |

**Primary backends**:
- Well-tested, production-ready
- Used by wgpu on all platforms
- Full feature support

**Secondary backends**:
- Less complete feature coverage
- Use for specific scenarios (GLSL for legacy OpenGL, WGSL for debugging)
- May have quirks or missing features

---

## SPIR-V Backend (naga::back::spv)

**Target**: Vulkan, OpenCL
**Output**: Binary format (SPIR-V instructions)
**Platform**: Cross-platform (primary Vulkan target)

### Basic Usage

```rust
use naga::back::spv;

let mut spv_binary = Vec::new();
let options = spv::Options::default();
let pipeline_options = spv::PipelineOptions {
    shader_stage: naga::ShaderStage::Fragment,
    entry_point: "main_fs".to_string(),
};

spv::write_vec(
    &module,
    &module_info,
    &options,
    Some(&pipeline_options),
    &mut spv_binary,
)?;

// spv_binary is Vec<u32> - SPIR-V binary ready for Vulkan
```

### Options (spv::Options)

**Key fields**:
- `lang_version: (u8, u8)` - SPIR-V version (e.g., `(1, 5)` for SPIR-V 1.5)
  - Default: `(1, 3)` (compatible with Vulkan 1.1+)
  - Use higher versions for newer features (requires newer Vulkan)

- `flags: WriterFlags` - Control output behavior
  - `WriterFlags::DEBUG` - Include debug info (OpName, OpLine, etc.)
  - `WriterFlags::ADJUST_COORDINATE_SPACE` - Convert coordinate systems
  - Default: `WriterFlags::empty()`

- `capabilities: Option<Capabilities>` - Required SPIR-V capabilities
  - Determines what SPIR-V extensions to use
  - If `None`, inferred from module

- `binding_map: BindingMap` - Map Naga bindings → SPIR-V descriptor sets
  - Customize resource binding layout
  - Default: Identity mapping

- `bounds_check_policies: BoundsCheckPolicies` - Array bounds checking
  - `ReadZeroSkipWrite` - OOB reads return 0, OOB writes are skipped
  - `Unchecked` - No bounds checks (UB if OOB)
  - Default: `ReadZeroSkipWrite`

- `zero_initialize_workgroup_memory: ZeroInitializeWorkgroupMemoryMode`
  - Controls if workgroup memory is zeroed
  - `None` - Don't initialize
  - `Polyfill` - Emit initialization code
  - Default: `None`

### Common Patterns

**Minimal (defaults)**:
```rust
let options = spv::Options::default();
spv::write_vec(&module, &module_info, &options, None, &mut output)?;
```

**Debug build (include names)**:
```rust
let options = spv::Options {
    flags: spv::WriterFlags::DEBUG,
    ..Default::default()
};
```

**Specific SPIR-V version**:
```rust
let options = spv::Options {
    lang_version: (1, 5),  // SPIR-V 1.5 (Vulkan 1.2+)
    ..Default::default()
};
```

### Gotchas

**SPIR-V is binary, not human-readable**:
- Use `spirv-dis` tool to disassemble for inspection
- Or use DOT backend for visualization

**Binding layout must match Vulkan descriptor sets**:
- WGSL `@group(X) @binding(Y)` → SPIR-V descriptor set X, binding Y
- If layout doesn't match pipeline, shader won't bind correctly

**Coordinate space differences**:
- WGSL/Naga uses Y-up, clip space [-1, 1] for both X and Y
- Vulkan uses Y-down clip space
- Use `WriterFlags::ADJUST_COORDINATE_SPACE` if needed

---

## MSL Backend (naga::back::msl)

**Target**: Metal (macOS, iOS, tvOS)
**Output**: Text (Metal Shading Language source code)
**Platform**: Apple platforms only

### Basic Usage

```rust
use naga::back::msl;

let mut msl_source = String::new();
let options = msl::Options::default();
let pipeline_options = msl::PipelineOptions {
    allow_and_force_point_size: true,
};

msl::Writer::new(&mut msl_source)
    .write(&module, &module_info, &options, &pipeline_options)?;

// msl_source is String - MSL shader source ready for Metal compiler
```

### Options (msl::Options)

**Key fields**:
- `lang_version: (u8, u8)` - MSL version (e.g., `(2, 4)` for MSL 2.4)
  - Default: `(2, 0)` (Metal 2.0, macOS 10.13+)
  - Newer versions: `(2, 4)` = macOS 11+, iOS 14+

- `per_entry_point_map: EntryPointResourceMap` - Binding mappings per entry point
  - Maps Naga resources → Metal argument buffer indices
  - Required because Metal has flat binding model (no descriptor sets)
  - Default: Empty (may cause binding errors if not configured)

- `inline_samplers: Vec<InlineSampler>` - Inline sampler configurations
  - Metal supports compile-time sampler definitions
  - Reduces runtime overhead

- `bounds_check_policies: BoundsCheckPolicies` - Array bounds checking
  - Same as SPIR-V backend

- `zero_initialize_workgroup_memory: bool` - Zero workgroup memory
  - Metal doesn't guarantee zero-init
  - Set to `true` to polyfill initialization
  - Default: `false`

- `spirv_cross_compatibility: bool` - SPIRV-Cross linking compatibility
  - Emit MSL compatible with SPIRV-Cross output (for multi-stage linking)
  - Rarely needed (only if mixing Naga + SPIRV-Cross shaders)
  - Default: `false`

- `fake_missing_bindings: bool` - Generate invalid MSL on missing bindings
  - For debugging (don't panic, emit placeholder)
  - Default: `false`

### Pipeline Options (msl::PipelineOptions)

**Per-translation configuration** (not per-module):
- `allow_and_force_point_size: bool` - Control point size output
  - Metal requires `[[point_size]]` for point primitives
  - Set to `true` if rendering points

### Common Patterns

**Minimal (macOS 10.13+)**:
```rust
let options = msl::Options::default();
let pipeline_options = msl::PipelineOptions::default();
msl::Writer::new(&mut output)
    .write(&module, &module_info, &options, &pipeline_options)?;
```

**Newer Metal version (macOS 11+)**:
```rust
let options = msl::Options {
    lang_version: (2, 4),
    ..Default::default()
};
```

**With zero-init workgroup memory**:
```rust
let options = msl::Options {
    zero_initialize_workgroup_memory: true,
    ..Default::default()
};
```

### Gotchas

**Binding model is flat, not hierarchical**:
- WGSL has `@group(X) @binding(Y)` (two-level hierarchy)
- Metal has single binding index
- Must map via `per_entry_point_map` or wgpu handles automatically

**Coordinate space differs from Vulkan**:
- Metal uses Y-up, same as Naga IR
- No coordinate adjustment needed (unlike Vulkan)

**Entry point naming**:
- MSL function names must match entry point strings
- wgpu handles this automatically

**External textures**:
- Naga lowers external textures → 3 `texture2d<float>` + params buffer
- Only relevant for video/camera input (WGSL extension)

---

## HLSL Backend (naga::back::hlsl)

**Target**: DirectX (D3D11, D3D12)
**Output**: Text (HLSL source code)
**Platform**: Windows primarily (also cross-platform via DXC)

### Basic Usage

```rust
use naga::back::hlsl;

let mut hlsl_source = String::new();
let options = hlsl::Options::default();

hlsl::Writer::new(&mut hlsl_source, &options)
    .write(&module, &module_info)?;

// hlsl_source is String - HLSL shader source
```

### Options (hlsl::Options)

**Key fields**:
- `shader_model: hlsl::ShaderModel` - Target shader model version
  - `ShaderModel::V5_0` - D3D11 (default)
  - `ShaderModel::V5_1` - D3D12
  - `ShaderModel::V6_0+` - Modern DX12 features

- `binding_map: hlsl::BindingMap` - Map Naga bindings → HLSL registers
  - `t#` - Texture/buffer resources
  - `s#` - Samplers
  - `u#` - UAVs (unordered access)
  - `b#` - Constant buffers

- `fake_missing_bindings: bool` - Don't panic on missing bindings

### Gotchas

**Register spaces differ from WGSL groups**:
- WGSL `@group(X) @binding(Y)` → HLSL register mapping
- Must configure `binding_map` or use wgpu defaults

**Shader model affects features**:
- SM 5.0 = D3D11 baseline
- SM 6.0+ = DXR, mesh shaders, etc.

---

## GLSL Backend (naga::back::glsl)

**Target**: OpenGL 3.3+, OpenGL ES 3.0+
**Output**: Text (GLSL source code)
**Platform**: Cross-platform (legacy compatibility)

### Basic Usage

```rust
use naga::back::glsl;

let mut glsl_source = String::new();
let options = glsl::Options::default();
let pipeline_options = glsl::PipelineOptions {
    shader_stage: naga::ShaderStage::Fragment,
    entry_point: "main_fs".to_string(),
    multiview: None,
};

glsl::Writer::new(
    &mut glsl_source,
    &module,
    &module_info,
    &options,
    &pipeline_options,
    naga::proc::BoundsCheckPolicies::default(),
)?.write()?;
```

### Gotchas

**Vulkan GLSL semantics only**:
- Backend targets Vulkan-flavored GLSL (GLSL 440+ with Vulkan extensions)
- Not compatible with legacy OpenGL GLSL (no `gl_FragColor`, etc.)

**Secondary support**:
- Less tested than SPIR-V/MSL/HLSL
- May have missing features or bugs

**Use SPIR-V instead if possible**:
- Vulkan supports SPIR-V natively
- GLSL backend mainly for OpenGL compatibility layer

---

## Translation Fidelity

### What Translates Well

**Type system**:
- Scalars (bool, i32, u32, f32, f16)
- Vectors (vec2, vec3, vec4)
- Matrices (mat2x2, mat4x4, etc.)
- Structs
- Arrays (fixed-size and runtime-sized)

**Operations**:
- Arithmetic (+, -, *, /, %)
- Comparison (<, >, ==, !=)
- Logical (&&, ||, !)
- Bitwise (&, |, ^, <<, >>)

**Control flow**:
- if/else
- for/while loops
- switch/case (WGSL → SPIR-V/MSL/HLSL)
- break/continue/return

**Built-ins**:
- Math functions (sin, cos, sqrt, pow, etc.)
- Vector operations (dot, cross, normalize, length, etc.)
- Texture sampling (textureSample, textureLoad, etc.)
- Atomics (atomicAdd, atomicMax, etc.)

### Platform Differences

**Feature availability**:
- **f64 (double)**: Not supported on all platforms (mobile GPUs often lack)
- **Subgroup operations**: Vulkan ✅, Metal ⚠️ (some), DX12 ✅
- **Int64 atomics**: Vulkan ✅, Metal ❌, DX12 ✅
- **Ray tracing**: Vulkan/DX12 only (SPIR-V/HLSL backends)

**Coordinate systems**:
- **Clip space Y**: Vulkan inverted, Metal/DX normal
- **Texture coords**: Vulkan Y-down, Metal Y-up
- **Depth range**: Vulkan [0,1], OpenGL [-1,1]

**Binding models**:
- **Vulkan (SPIR-V)**: Descriptor sets + bindings (group, binding)
- **Metal (MSL)**: Flat buffer/texture indices
- **DX (HLSL)**: Register spaces (t#, s#, u#, b#)

**Workgroup memory**:
- Metal doesn't zero-initialize by default
- Use `zero_initialize_workgroup_memory` option if needed

### What Gets Lost

**Precision**:
- WGSL `f32` → MSL `float` (may be `half` on mobile for performance)
- Backend may lower precision (opt-in via options)

**Debug info**:
- Variable names preserved if `WriterFlags::DEBUG` enabled (SPIR-V)
- Otherwise, names may be mangled or lost

**Unsupported features**:
- If Naga IR uses capability not in backend, translation fails
- Example: Int64 atomics in WGSL → MSL (not supported, fails)

---

## Error Handling

### Backend Errors

```rust
match msl::Writer::new(&mut output).write(&module, &module_info, &options, &pipeline_options) {
    Ok(translation_info) => {
        // translation_info includes entry point details
        println!("Translation succeeded");
    }
    Err(e) => {
        eprintln!("MSL translation failed: {:?}", e);
        // Backend error (not validation error)
    }
}
```

**Backend errors** ≠ **validation errors**:
- Validation checks IR correctness
- Backend translation checks platform compatibility
- Module can pass validation but fail translation (unsupported feature)

**Example backend failures**:
- Using f64 when target doesn't support doubles
- Using subgroup ops without required capabilities
- Binding index out of range for platform

### When Validation Passes But Translation Fails

**Scenario**: Shader validates with `Capabilities::all()` but MSL backend fails

**Cause**: Naga IR includes feature not supported by Metal

**Solution**:
1. Validate with platform-specific capabilities (not `Capabilities::all()`)
2. Or handle backend error and report to user

**Pattern**:
```rust
// Validate for Metal capabilities (conservative)
let metal_caps = Capabilities::default();  // Metal-compatible subset
let mut validator = Validator::new(ValidationFlags::all(), metal_caps);
let module_info = validator.validate(&module)?;

// Now translation should succeed (if validation passed)
let mut msl_output = String::new();
msl::Writer::new(&mut msl_output)
    .write(&module, &module_info, &msl::Options::default(), &msl::PipelineOptions::default())?;
```

---

## Practical Workflows

### Workflow 1: WGSL → SPIR-V (Vulkan)

```rust
// Parse
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Validate
let module_info = naga::valid::Validator::new(
    naga::valid::ValidationFlags::all(),
    naga::valid::Capabilities::all(),
).validate(&module)?;

// Translate to SPIR-V
let mut spv_binary = Vec::new();
naga::back::spv::write_vec(
    &module,
    &module_info,
    &naga::back::spv::Options::default(),
    None,
    &mut spv_binary,
)?;

// spv_binary ready for vkCreateShaderModule()
```

### Workflow 2: WGSL → MSL (Metal)

```rust
// Parse + validate (same as above)
let module = naga::front::wgsl::parse_str(wgsl_source)?;
let module_info = validator.validate(&module)?;

// Translate to MSL
let mut msl_source = String::new();
naga::back::msl::Writer::new(&mut msl_source).write(
    &module,
    &module_info,
    &naga::back::msl::Options {
        lang_version: (2, 4),  // macOS 11+
        ..Default::default()
    },
    &naga::back::msl::PipelineOptions::default(),
)?;

// msl_source ready for MTLLibrary compilation
```

### Workflow 3: Multi-Platform (via wgpu)

```rust
// wgpu handles backend selection automatically
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("my_shader"),
    source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
});

// wgpu internally:
// 1. Parses WGSL via Naga frontend
// 2. Validates via Naga validator
// 3. Translates via appropriate backend (SPIR-V for Vulkan, MSL for Metal, etc.)
// 4. Compiles platform shader via driver
```

---

## Open Questions (for Discovery Phase)

**Q1**: What does vibesurfer's WGSL translate to on macOS?
- Inspect MSL output for compute and fragment shaders
- Check if binding layout needs adjustment

**Q2**: What are actual translation costs?
- Benchmark WGSL → SPIR-V vs WGSL → MSL
- Measure parse + validate + translate pipeline

**Q3**: Do any vibesurfer shaders use platform-incompatible features?
- Test with platform-specific capabilities
- Document if features need fallbacks

**Q4**: Can we cache translated shaders?
- SPIR-V is stable (can cache binary)
- MSL source may change across Naga versions
- Test cache invalidation strategy

**Q5**: How to debug backend translation failures?
- Use DOT backend to visualize IR
- Use naga-cli to test translation offline
- Compare Naga output to hand-written shader

---

## References

**Primary documentation**:
- [naga::back module](https://docs.rs/naga/latest/naga/back/)
- [SPIR-V backend](https://docs.rs/naga/latest/naga/back/spv/) (cached: `.webcache/naga/back-spv.html`)
- [MSL backend](https://docs.rs/naga/latest/naga/back/msl/) (cached: `.webcache/naga/back-msl.html`)
- [SPIR-V Options](https://docs.rs/naga/latest/naga/back/spv/struct.Options.html) (cached: `.webcache/naga/spv-options.html`)
- [MSL Options](https://docs.rs/naga/latest/naga/back/msl/struct.Options.html) (cached: `.webcache/naga/msl-options.html`)

**Backend modules**:
- `naga::back::spv` - SPIR-V binary writer
- `naga::back::msl` - MSL text writer
- `naga::back::hlsl` - HLSL text writer
- `naga::back::glsl` - GLSL text writer
- `naga::back::wgsl` - WGSL text writer (debug)
- `naga::back::dot` - GraphViz DOT writer (visualization)

**Tools**:
- `naga-cli` - Test translation from command line
- `spirv-dis` - Disassemble SPIR-V binary (from SPIRV-Tools)
- `xcrun metal` - Compile MSL on macOS (test output validity)

---

**Created**: October 2025
**Last updated**: October 2025
**Status**: Priority 2 complete (translation and backends documented)
