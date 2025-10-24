# Naga Validation - Error Detection and Debugging

**Purpose**: Understanding Naga's validation system, error types, and debugging strategies

**Audience**: AI agents writing and debugging shaders in Rust projects

**Key insight**: Validation is mandatory (backends require `ModuleInfo`). Errors are typed, spanned, and debuggable. Configure strictness via `ValidationFlags` and `Capabilities`.

---

## What Validation Does

**Naga validation** = Type-checking + platform compatibility checks for shader IR

**Purpose**: Catch errors before backend translation or GPU execution
- Type correctness (operands match operation requirements)
- Resource binding validity (textures/uniforms/storage match layout)
- Control flow correctness (no invalid branches, guaranteed returns)
- Platform capability compatibility (features supported by target GPU)
- Memory layout validity (host-shareable structs meet alignment rules)

**When it runs**:
- **Via wgpu**: Automatically when calling `device.create_shader_module(...)`
- **Standalone**: Explicitly via `Validator::new(...).validate(&module)`

**Output**: `Result<ModuleInfo, WithSpan<ValidationError>>`
- **Success**: `ModuleInfo` (metadata required by backends)
- **Failure**: Error with source location and diagnostic info

---

## Core API: naga::valid::Validator

### Creation

```rust
use naga::valid::{Validator, ValidationFlags, Capabilities};

let mut validator = Validator::new(
    ValidationFlags::all(),  // Which checks to perform
    Capabilities::all(),     // Which GPU features to allow
);
```

**ValidationFlags**: Which checks to enable
- `EXPRESSIONS` - Expression type checking
- `BLOCKS` - Statement and block validation
- `CONTROL_FLOW_UNIFORMITY` - Uniform control flow requirements
- `STRUCT_LAYOUTS` - Host-shareable struct alignment/size
- `CONSTANTS` - Constant expression validation
- `BINDINGS` - Resource binding attributes (group, binding, location)

**Capabilities**: GPU feature support
- Controls what shader features are allowed (subgroup ops, atomics, etc.)
- Platform-specific: Metal has different capabilities than Vulkan
- Use `Capabilities::all()` for permissive validation
- Use platform-specific capabilities for target compatibility

**Pattern**: Most code uses `ValidationFlags::all()` + `Capabilities::all()`

### Validation

```rust
let module_info = validator.validate(&module)?;
```

**Returns**:
- `Ok(ModuleInfo)` - Module is valid, info needed for backend
- `Err(WithSpan<ValidationError>)` - Validation failed with error + location

**Critical**: Backends require `ModuleInfo` from validation
- `ModuleInfo` contains metadata: expression types, uniformity analysis, resource usage
- Cannot skip validation and pass to backend (will panic or produce incorrect output)
- Even if you "trust" your shader, validation is cheap (< 1ms for typical shaders)

### Configuration (Optional)

```rust
validator
    .subgroup_stages(ShaderStages::COMPUTE)  // Which stages allow subgroup ops
    .subgroup_operations(SubgroupOperationSet::BASIC);  // Which subgroup ops allowed
```

**Subgroup configuration**:
- Controls validation of subgroup operations (WGPU extension)
- Platform-specific (not all GPUs support all subgroup ops)
- Only needed if shader uses subgroup intrinsics

### Reusing Validator

```rust
validator.reset();  // Clear internal state
let module_info2 = validator.validate(&another_module)?;
```

**Pattern**: Create one `Validator`, reuse for multiple modules
- Call `reset()` between validations
- Avoids re-allocating internal data structures

---

## Validation Error Types

### Error Structure

```rust
pub enum ValidationError {
    Type { handle, name, source: TypeError },
    Function { handle, name, source: FunctionError },
    GlobalVariable { handle, name, source: GlobalVariableError },
    EntryPoint { stage, name, source: EntryPointError },
    Constant { handle, name, source: ConstantError },
    ConstExpression { handle, source: ConstExpressionError },
    // ... more variants
}
```

**Pattern**: Hierarchical errors
- Top level: What IR element failed (type, function, global, entry point)
- `handle`: IR reference to failing element (e.g., `Handle<Function>`)
- `name`: Human-readable name (from shader source)
- `source`: Detailed sub-error (what specifically went wrong)

**WithSpan wrapper**: Errors include source location
```rust
pub struct WithSpan<E> {
    inner: E,  // The ValidationError
    spans: Vec<(Span, String)>,  // Source locations + context
}
```

**Span**: Source code location (file, line, column range)
- Allows pointing to exact shader line that caused error
- Critical for error messages and IDE integration

### Common Error Categories

**Type errors** (`TypeError`):
- Mismatched types in operations (e.g., `vec3 + float`)
- Invalid type for operation (e.g., bitwise on float)
- Type not supported by target (e.g., f64 on platforms without double support)

**Function errors** (`FunctionError`):
- Missing return statement
- Invalid control flow (unreachable code, divergent returns)
- Expression errors (invalid operations in function body)

**Entry point errors** (`EntryPointError`):
- Invalid signature for shader stage (wrong inputs/outputs)
- Missing required builtin variables
- Resource binding conflicts

**Binding errors**:
- Duplicate binding indices (two resources at same `@group(0) @binding(0)`)
- Invalid location attributes (fragment outputs)
- Resource type mismatch (bound as texture but used as buffer)

**Constant errors** (`ConstantError`):
- Non-constant expression in const context
- Const evaluation failure (overflow, divide by zero)

**Layout errors** (`LayoutError`):
- Struct doesn't meet host-shareable alignment rules
- Invalid memory layout for uniform/storage buffer

---

## Error Handling Patterns

### Pattern 1: Validate and Report

```rust
let module = naga::front::wgsl::parse_str(wgsl_source)?;

match validator.validate(&module) {
    Ok(module_info) => {
        println!("Shader valid");
        // Proceed to backend translation
    }
    Err(validation_error) => {
        eprintln!("Validation failed: {}", validation_error);
        // validation_error includes spans for debugging
    }
}
```

### Pattern 2: Validation in CI/Linting

```rust
fn lint_shader(path: &Path) -> Result<(), String> {
    let source = std::fs::read_to_string(path)?;
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    naga::valid::Validator::new(
        ValidationFlags::all(),
        Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| format!("Validation error in {}: {}", path.display(), e))?;

    Ok(())
}
```

### Pattern 3: Platform-Specific Validation

```rust
// Validate for Metal (macOS/iOS)
let metal_caps = Capabilities::default();  // Conservative Metal capabilities
let mut validator = Validator::new(ValidationFlags::all(), metal_caps);

let module_info = validator.validate(&module)?;

// Now translate to MSL
let msl_output = naga::back::msl::Writer::new(&mut output)
    .write(&module, &module_info, &msl::Options::default())?;
```

**Why platform-specific**:
- Different GPUs support different features
- Validating with `Capabilities::all()` may pass shaders that fail on target
- Better to catch incompatibilities at validation time than runtime

---

## Debugging Validation Errors

### Using Spans for Error Location

```rust
match validator.validate(&module) {
    Err(error) => {
        // WithSpan<ValidationError> includes source locations
        eprintln!("Validation error: {}", error);

        // If you have source code, map spans back to lines
        for (span, context) in error.spans() {
            eprintln!("  at {:?}: {}", span, context);
        }
    }
    Ok(_) => { /* ... */ }
}
```

**Span structure**:
```rust
pub struct Span {
    start: u32,  // Byte offset in source
    end: u32,    // Byte offset in source
}
```

**SourceLocation** (human-readable):
```rust
pub struct SourceLocation {
    line_number: usize,
    line_position: usize,
    // + offset, length
}
```

**Pattern**: Convert `Span` → `SourceLocation` for error messages
- Naga provides helpers for this (see `codespan-reporting` integration)
- IDEs can use spans to highlight error locations

### Common Validation Failures and Fixes

**Error**: "Type X cannot be used in uniform buffer"
- **Cause**: Type doesn't meet host-shareable layout rules
- **Fix**: Add padding, use `std140` layout, or change buffer type to storage

**Error**: "Entry point function must return void"
- **Cause**: Compute/vertex shader returning value incorrectly
- **Fix**: Use output parameters or builtin outputs instead of return value

**Error**: "Expression X is not constant"
- **Cause**: Non-const expression used in const context (array size, etc.)
- **Fix**: Use `const` or `override` declarations, not runtime expressions

**Error**: "Binding X conflicts with binding Y"
- **Cause**: Two resources have same `@group(G) @binding(B)`
- **Fix**: Assign unique binding indices per group

**Error**: "Capability X not supported"
- **Cause**: Shader uses feature not in validator's `Capabilities`
- **Fix**: Enable capability or rewrite shader to avoid feature

---

## Validation vs Driver Errors

**Naga validation errors**:
- Happen during `validator.validate(&module)` or `device.create_shader_module(...)`
- Structured errors with types and spans
- Caught before GPU execution
- Examples: type mismatch, binding conflict, invalid control flow

**Driver errors**:
- Happen during GPU execution or driver compilation
- Platform-specific, opaque error messages
- Not caught by Naga (passed validation but failed on GPU)
- Examples: out-of-memory, driver bug, unsupported hardware feature

**How to distinguish**:
- If error occurs during `create_shader_module()`, it's likely Naga validation
- If error occurs during `queue.submit()` or render/compute pass, it's likely driver
- Naga errors have structured types (`ValidationError::Type`, etc.)
- Driver errors are often strings from GPU API (Vulkan, Metal, D3D)

**wgpu error surfacing**:
- wgpu wraps Naga validation errors in `CreateShaderModuleError`
- Includes both Naga validation errors and backend compilation errors
- Check `error.source()` to see if it's from Naga or driver

---

## Performance Characteristics

**Validation cost**: Negligible for typical shaders
- Most shaders: < 1ms validation time
- Even large compute shaders (1000+ lines): < 10ms
- Cost dominated by type-checking expressions and control flow analysis

**Should you validate at runtime?**
- **Yes for development**: Catch errors early with good error messages
- **Yes for user-generated shaders**: Untrusted input must be validated
- **Optional for production**: Pre-validated shaders can skip runtime validation
  - But validation is so cheap, usually not worth the complexity

**Build-time validation**:
- Use `naga-cli` or build script to validate shaders offline
- Catch errors during build instead of runtime
- Useful for shipping pre-compiled shaders (SPIR-V, MSL)

**Caching validated modules**:
- `ModuleInfo` is tied to specific `Module` instance
- Don't cache `ModuleInfo` across runs (internal references invalid)
- If caching, cache source or SPIR-V, not validated IR

---

## Integration with wgpu

### wgpu Automatic Validation

```rust
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("my_shader"),
    source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
});
```

**What happens internally**:
1. wgpu parses WGSL via `naga::front::wgsl::parse_str()`
2. wgpu validates via `naga::valid::Validator::new(...).validate()`
3. If validation passes, wgpu translates to platform shader (SPIR-V, MSL, etc.)
4. Backend compiles and creates `ShaderModule`

**Error handling**:
```rust
let shader = match device.create_shader_module(desc) {
    Ok(module) => module,
    Err(e) => {
        eprintln!("Shader creation failed: {}", e);
        // e includes Naga validation errors if validation failed
        return;
    }
};
```

**Note**: wgpu doesn't expose `ModuleInfo` publicly
- `ModuleInfo` is used internally by wgpu backends
- If you need `ModuleInfo`, use Naga directly

### Bypassing Validation (Unsafe)

```rust
// UNSAFE: Skip Naga validation and use shader directly
let shader = unsafe {
    device.create_shader_module_trusted(
        desc,
        wgpu::ShaderRuntimeChecks::unchecked(),
    )
};
```

**When to bypass**:
- Pre-validated shaders (validated at build time)
- Performance-critical path (but validation is already fast)
- Debugging backend issues (isolate Naga vs driver errors)

**Risks**:
- Invalid shader may crash driver or produce undefined behavior
- No structured error messages
- Hard to debug issues

**Recommendation**: Only bypass in production after extensive testing

---

## Gotchas

**Validation is not optional**:
- Backends require `ModuleInfo` from `Validator.validate()`
- Attempting to skip validation = panic or incorrect output

**ValidationFlags don't affect ModuleInfo**:
- Even with minimal flags, `ModuleInfo` is still computed
- Flags control what errors are reported, not what analysis is done
- Skipping flags trades error reporting for minimal speed gain (not worth it)

**Capabilities are conservative**:
- `Capabilities::all()` allows everything Naga supports
- Actual GPU may not support all capabilities
- Better to use platform-specific capabilities for validation

**Spans require source tracking**:
- If parsing without source (`Module` constructed manually), spans are invalid
- Error messages won't point to source locations
- Always parse from source (WGSL, SPIR-V with debug info) when possible

**Validation order matters**:
- Validate before translation (can't validate after backend writes)
- Some backends add extra checks beyond Naga validation
- Backend errors != Naga validation errors

**wgpu hides ModuleInfo**:
- If using wgpu APIs, you don't get direct access to `ModuleInfo`
- wgpu uses it internally for backend translation
- If you need `ModuleInfo`, use Naga standalone

---

## Open Questions (for Discovery Phase)

**Q1**: What are the most common validation errors when writing WGSL for vibesurfer?
- Test compute shaders (sphere projection, terrain gen)
- Test fragment shaders (wireframe, fog, lighting)
- Catalog actual error messages

**Q2**: How do Naga validation errors appear in wgpu error messages?
- Trigger validation failure in vibesurfer shader
- Inspect `CreateShaderModuleError` structure
- Document error surfacing pattern

**Q3**: What's the actual validation cost for vibesurfer shaders?
- Benchmark `validator.validate()` on real shaders
- Measure parse + validate pipeline
- Determine if runtime validation is acceptable

**Q4**: What platform-specific capabilities should vibesurfer use?
- macOS Metal capabilities (primary target)
- Vulkan/SPIR-V capabilities (future targets)
- Document capability differences

**Q5**: Can we use naga-cli in build pipeline?
- Test `naga shader.wgsl --validate` workflow
- Integrate into `cargo xtask` or pre-commit hook
- Catch shader errors at build time

---

## References

**Primary documentation**:
- [naga::valid module](https://docs.rs/naga/latest/naga/valid/) (cached: `.webcache/naga/valid-module.html`)
- [Validator struct](https://docs.rs/naga/latest/naga/valid/struct.Validator.html) (cached: `.webcache/naga/validator-struct.html`)
- [ValidationFlags](https://docs.rs/naga/latest/naga/valid/struct.ValidationFlags.html) (cached: `.webcache/naga/validation-flags.html`)
- [ValidationError](https://docs.rs/naga/latest/naga/valid/enum.ValidationError.html) (cached: `.webcache/naga/validation-error.html`)
- [wgpu::Device](https://docs.rs/wgpu/latest/wgpu/struct.Device.html) (cached: `.webcache/naga/wgpu-device.html`)

**Key types**:
- `naga::valid::Validator` - Validation engine
- `naga::valid::ModuleInfo` - Validation output (required by backends)
- `naga::valid::ValidationError` - Error hierarchy
- `naga::WithSpan<E>` - Error with source location
- `naga::Span` / `naga::SourceLocation` - Source code positions

**Tools**:
- `naga-cli` - Validate shaders from command line
- `codespan-reporting` - Pretty-print errors with source context

---

**Created**: October 2025
**Last updated**: October 2025
**Status**: Priority 1 complete (validation and error handling documented)
