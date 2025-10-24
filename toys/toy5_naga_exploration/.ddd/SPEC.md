# Toy: Naga Exploration Specification

Validate research learnings by testing Naga standalone with real vibesurfer shaders

---

## Overview

**What it does**: Standalone tool that parses, validates, and translates WGSL shaders using Naga, measuring performance and inspecting output to validate research findings.

**Key principles**:
- Test theory from research phase (validation errors, translation output, performance)
- Use real vibesurfer shaders (not synthetic examples)
- Measure actual behavior (not assumptions)
- Catalog findings for reference update

**Scope**: Isolates Naga usage from wgpu (standalone library usage)
- Parse WGSL → Naga IR
- Validate with different capability sets
- Translate to MSL (macOS target)
- Measure performance
- Intentionally trigger errors to inspect messages

**Integration context**: Reads shaders from vibesurfer or toy4, outputs translation results and measurements

---

## Data Model

### Input: WGSL Shader Source

**Location**: vibesurfer shader files or toy4 shaders
- `vibesurfer/src/shaders/*.wgsl` (if they exist)
- `toys/toy4_spherical_chunks/src/*.wgsl` (embedded in Rust)
- Or inline test shaders for error scenarios

**Example** (simple fragment shader):
```wgsl
@fragment
fn main_fs(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv.x, uv.y, 0.0, 1.0);
}
```

### Output: Translation Results

**MSL Output** (Metal Shading Language):
```metal
#include <metal_stdlib>
using namespace metal;

struct main_fsOutput {
    float4 member [[color(0)]];
};

fragment main_fsOutput main_fs(
    float2 uv [[user(loc0)]]
) {
    main_fsOutput output;
    output.member = float4(uv.x, uv.y, 0.0, 1.0);
    return output;
}
```

**Validation Info**:
```rust
struct ValidationResult {
    success: bool,
    module_info: Option<naga::valid::ModuleInfo>,
    error: Option<String>,
    error_spans: Vec<(Span, String)>,
}
```

**Performance Metrics**:
```rust
struct PipelineMetrics {
    parse_time_us: u64,
    validate_time_us: u64,
    translate_time_us: u64,
    total_time_us: u64,
}
```

---

## Core Operations

### Operation 1: Parse WGSL

**Syntax**:
```rust
parse_wgsl(source: &str) -> Result<naga::Module, ParseError>
```

**Behavior**:
- Call `naga::front::wgsl::parse_str(source)`
- Return `Module` (IR) on success
- Return `ParseError` with spans on failure

**Example**:
```rust
let wgsl_source = include_str!("test_shader.wgsl");
let module = parse_wgsl(wgsl_source)?;
println!("Parsed successfully: {} types, {} functions",
    module.types.len(), module.functions.len());
```

**Validation**:
- Valid WGSL → Ok(Module)
- Syntax error → Err with span pointing to error location
- Empty source → Err (no entry points)

### Operation 2: Validate Module

**Syntax**:
```rust
validate_module(
    module: &naga::Module,
    capabilities: naga::valid::Capabilities
) -> Result<naga::valid::ModuleInfo, ValidationError>
```

**Parameters**:
- `capabilities`: `all()` (permissive), `default()` (conservative), or platform-specific

**Behavior**:
- Create `Validator` with `ValidationFlags::all()` and given capabilities
- Call `validator.validate(module)`
- Return `ModuleInfo` on success
- Return `ValidationError` with spans on failure

**Examples**:

*Success case*:
```rust
let module_info = validate_module(&module, Capabilities::all())?;
println!("Validation passed");
```

*Failure case* (intentional error):
```rust
// Shader with type mismatch
let bad_wgsl = "@fragment fn main() -> @location(0) vec4<f32> { return 1.0; }";
let module = parse_wgsl(bad_wgsl)?;
match validate_module(&module, Capabilities::all()) {
    Err(e) => {
        println!("Validation error: {}", e);
        for (span, ctx) in e.spans() {
            println!("  at {:?}: {}", span, ctx);
        }
    }
    Ok(_) => unreachable!(),
}
```

**Validation**:
- Valid module → Ok(ModuleInfo)
- Type error → Err with specific error type and spans
- Missing capability → Err with capability error

### Operation 3: Translate to MSL

**Syntax**:
```rust
translate_to_msl(
    module: &naga::Module,
    module_info: &naga::valid::ModuleInfo,
    lang_version: (u8, u8)
) -> Result<String, BackendError>
```

**Parameters**:
- `lang_version`: `(2, 0)` (Metal 2.0, macOS 10.13+) or `(2, 4)` (Metal 2.4, macOS 11+)

**Behavior**:
- Create `msl::Writer`
- Call `writer.write()` with options
- Return MSL source string on success
- Return backend error on failure

**Example**:
```rust
let msl_source = translate_to_msl(&module, &module_info, (2, 4))?;
println!("MSL output:\n{}", msl_source);
```

**Validation**:
- Valid module + Metal-compatible features → Ok(String)
- Unsupported feature (e.g., f64) → Err(backend error)

### Operation 4: Measure Pipeline Performance

**Syntax**:
```rust
benchmark_pipeline(source: &str, iterations: u32) -> PipelineMetrics
```

**Behavior**:
- Run parse + validate + translate `iterations` times
- Measure each stage with high-precision timer
- Return average times in microseconds

**Example**:
```rust
let metrics = benchmark_pipeline(wgsl_source, 100);
println!("Parse: {}μs, Validate: {}μs, Translate: {}μs, Total: {}μs",
    metrics.parse_time_us,
    metrics.validate_time_us,
    metrics.translate_time_us,
    metrics.total_time_us);
```

**Validation**:
- Total time should be parse + validate + translate (±1μs rounding)
- All times should be > 0

### Operation 5: Test Capability Constraints

**Syntax**:
```rust
test_capabilities(module: &naga::Module) -> CapabilityTestResult
```

**Behavior**:
- Validate with `Capabilities::all()` (permissive)
- Validate with `Capabilities::default()` (conservative)
- Compare results (do they differ?)
- Report which capabilities are required

**Example**:
```rust
let result = test_capabilities(&module);
println!("All caps: {}, Default caps: {}",
    result.all_caps_valid, result.default_caps_valid);
if !result.default_caps_valid {
    println!("Required capabilities: {:?}", result.required_caps);
}
```

**Validation**:
- If `all()` passes but `default()` fails → shader uses advanced features
- If both pass → shader is cross-platform compatible
- If both fail → shader has errors

---

## Test Scenarios

### Simple: Valid Fragment Shader

**Input**:
```wgsl
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
```

**Expected**:
- ✅ Parse succeeds
- ✅ Validate succeeds (all capabilities, default capabilities)
- ✅ Translate to MSL succeeds
- ✅ MSL includes `fragment` function returning `float4`
- ✅ Pipeline time < 5ms

### Complex: Real Vibesurfer Shader

**Input**: Use actual shader from toy4 (e.g., wireframe fragment shader with fog)

**Expected**:
- ✅ Parse succeeds
- ✅ Validate succeeds with appropriate capabilities
- ✅ Translate to MSL succeeds
- ✅ MSL output is readable and matches shader logic
- ✅ Pipeline time measured and documented

### Error: Type Mismatch

**Input**:
```wgsl
@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return 1.0; // Should return vec4, not scalar
}
```

**Expected**:
- ✅ Parse succeeds
- ❌ Validate fails with `ValidationError::Function`
- ✅ Error includes span pointing to `return 1.0;`
- ✅ Error message describes type mismatch

### Error: Missing Binding

**Input**:
```wgsl
@group(0) @binding(0) var my_texture: texture_2d<f32>;
@group(0) @binding(0) var my_sampler: sampler; // Duplicate binding

@fragment
fn main_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
```

**Expected**:
- ✅ Parse succeeds
- ❌ Validate fails with binding conflict error
- ✅ Error identifies both conflicting bindings

### Error: Unsupported Feature (Platform-Specific)

**Input**:
```wgsl
@fragment
fn main_fs() -> @location(0) vec4<f64> { // f64 not supported on Metal
    return vec4<f64>(1.0);
}
```

**Expected**:
- ✅ Parse succeeds
- ⚠️ Validate may pass with `Capabilities::all()` (if Naga supports f64 IR)
- ❌ Translate to MSL fails (Metal doesn't support f64)
- ✅ Backend error message explains feature not supported

---

## Success Criteria

### Research Validation

- [ ] Confirmed validation is mandatory (backends require `ModuleInfo`)
- [ ] Measured actual pipeline cost for real vibesurfer shaders
- [ ] Inspected MSL translation output (readable, correct)
- [ ] Triggered validation errors and inspected spans/messages
- [ ] Tested with different capability sets (all vs default vs Metal)

### Practical Measurements

- [ ] Parse time < 5ms for typical shaders
- [ ] Validate time < 5ms for typical shaders
- [ ] Translate time < 5ms for typical shaders
- [ ] Total pipeline time < 15ms for typical shaders

### Error Handling

- [ ] Validation errors include useful spans (byte offsets)
- [ ] Error messages are actionable (describe what's wrong)
- [ ] Backend errors distinguishable from validation errors

### Platform Compatibility

- [ ] Identified if vibesurfer shaders require specific capabilities
- [ ] Tested that `Capabilities::default()` works or documented required caps
- [ ] Verified MSL output compiles with Metal compiler (optional: `xcrun metal -c`)

### Documentation Updates

- [ ] Updated `naga-reference.md` with practical findings
- [ ] Documented actual error patterns observed
- [ ] Added performance measurements to reference
- [ ] Cataloged any surprises or gotchas not in research docs

---

## Constraints

**Platform**: macOS Apple Silicon (M1)
- MSL backend is primary target
- Can test SPIR-V backend but won't validate with Vulkan

**Time box**: 1-2 hours total
- Tool implementation: 30-60 min
- Testing with real shaders: 30-60 min
- Documentation updates: 30 min

**Scope limits**:
- Don't test all backends (focus on MSL)
- Don't test all error scenarios (sample key patterns)
- Don't build complete CLI tool (simple test harness OK)

---

## Non-Goals

**Not building**:
- Production shader validation tool (toy only)
- Complete CLI with argument parsing
- Integration with vibesurfer build system
- Comprehensive error catalog

**Not testing**:
- SPIR-V compilation with Vulkan drivers (no Vulkan on this machine)
- HLSL backend (Windows-specific)
- GLSL backend (secondary, not vibesurfer target)
- Custom frontends or IR manipulation

---

## Deliverables

**Code**:
- `toys/naga_exploration/src/main.rs` - Test harness
- `toys/naga_exploration/Cargo.toml` - Dependencies (naga with features)

**Documentation**:
- `toys/naga_exploration/.ddd/LEARNINGS.md` - What we discovered
- Updated `learnings/naga-reference.md` - Add practical findings

**Measurements**:
- Pipeline performance metrics (parse, validate, translate times)
- MSL output samples
- Error message samples

---

## Example Usage

```rust
// Test harness pseudocode
fn main() {
    // 1. Parse real shader
    let wgsl_source = include_str!("../../toy4_spherical_chunks/src/shaders/wireframe.wgsl");
    let module = parse_wgsl(wgsl_source).expect("Parse failed");

    // 2. Validate
    let module_info = validate_module(&module, Capabilities::default())
        .expect("Validation failed");

    // 3. Translate to MSL
    let msl_source = translate_to_msl(&module, &module_info, (2, 4))
        .expect("Translation failed");

    println!("MSL output:\n{}", msl_source);

    // 4. Benchmark
    let metrics = benchmark_pipeline(wgsl_source, 100);
    println!("Performance: {:?}", metrics);

    // 5. Test error scenarios
    test_validation_errors();
    test_capability_constraints(&module);
}
```

---

## Falsifiable Claims from Research

**Claim 1**: Validation cost is < 1ms for typical shaders
- **Test**: Benchmark validation on vibesurfer shaders
- **Falsified if**: Validation takes > 5ms consistently

**Claim 2**: Validation is mandatory (backends require ModuleInfo)
- **Test**: Attempt to call backend without validation
- **Falsified if**: Backend accepts Module without ModuleInfo

**Claim 3**: Errors include source spans for debugging
- **Test**: Trigger validation error, inspect WithSpan wrapper
- **Falsified if**: Errors don't include byte offsets or context

**Claim 4**: MSL output is readable and matches shader logic
- **Test**: Inspect MSL translation of real shader
- **Falsified if**: MSL is unreadable or semantically different

**Claim 5**: Platform capabilities matter for validation
- **Test**: Validate with all() vs default(), compare results
- **Falsified if**: No difference between capability sets

---

This spec defines the behavioral contract for the Naga exploration toy. Implementation should validate all claims and update the reference documentation with findings.
