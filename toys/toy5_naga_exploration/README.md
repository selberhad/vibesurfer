# Toy 5: Naga Exploration

Standalone validation of Naga shader translation pipeline with performance measurements

## Purpose

Validates research claims from `learnings/naga-reference.md` by testing Naga's parse/validate/translate pipeline with real measurements. Confirms that validation is mandatory, performance is negligible, and MSL output is production-ready. Built as infrastructure validation—not vibesurfer-specific, but uses vibesurfer as context for future shader work.

## Key Findings

**Performance (M1 MacBook Pro)**:
- Parse: 7μs | Validate: ~0μs | Translate: 4μs | **Total: 11μs**
- 3 orders of magnitude faster than research budget (15ms)
- Runtime validation is essentially free

**MSL Output**:
- 258 chars for simple fragment shader
- Production-ready Metal 2.4 code with proper headers
- No post-processing needed

**Validation**:
- Mandatory (backends require `ModuleInfo`)
- Works with both `Capabilities::all()` and `Capabilities::default()`
- Parse errors more comprehensive than expected (catches type mismatches)

## Core Test Pattern

```rust
// Parse
let module = naga::front::wgsl::parse_str(wgsl_source)?;

// Validate (required for backends)
let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
let module_info = validator.validate(&module)?;

// Translate to Metal
let mut msl_source = String::new();
msl::Writer::new(&mut msl_source)
    .write(&module, &module_info, &msl::Options::default(), &msl::PipelineOptions::default())?;
```

## Gotchas

- **ModuleInfo has no `is_empty()`**: Just check that validation returns Ok
- **Binding conflicts don't fail validation**: May be checked at runtime/backend
- **Type errors caught at parse time**: WGSL parser is stricter than expected
- **Some errors only appear at backend**: Validate + translate together to catch all issues

## Recommendations

**For vibesurfer**:
- Use `naga-cli` for offline shader testing (`naga shader.wgsl shader.metal`)
- Don't pre-compile for performance (11μs is negligible)
- Add shader validation to CI/build.rs
- Test full pipeline to catch backend-specific errors

## Quick Test

```bash
cargo run
```

Runs all tests: parse, validate, translate, benchmark (100 iterations), error scenarios.

## Integration

Not integrated with vibesurfer runtime—standalone validation toy. Findings documented in `learnings/naga-reference.md`.
