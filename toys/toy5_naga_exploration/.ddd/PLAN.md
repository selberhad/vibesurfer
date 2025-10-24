# Toy 5: Naga Exploration - Implementation Plan

**Goal**: Validate Naga research learnings with practical testing of parse/validate/translate pipeline

**Scope**: Minimal test harness (single file, ~200 lines) that exercises Naga standalone with real shaders

**Time box**: 1-2 hours total

**Methodology**: TDD - write test cases first, implement to pass, measure and document findings

---

## Overview

**What we're building**:
- Standalone Rust binary that exercises Naga API
- Tests parsing, validation, translation with real vibesurfer shaders
- Measures performance, inspects output, triggers errors
- Documents findings for reference update

**What we're NOT building**:
- Production tool or CLI framework
- Comprehensive error catalog
- Multi-backend support (MSL only)
- Integration with vibesurfer build system

**TDD approach**:
- Write assertion-based tests inline (not separate test module)
- Use `assert!`, `expect()`, pattern matching for validation
- Each step = Red (write test) → Green (implement) → Commit

---

## Step 1: Project Setup

### Goal
Create toy project with Naga dependency and basic structure

### Step 1.a: Initialize Project

**Tasks**:
1. Create `toys/toy5_naga_exploration/Cargo.toml`
2. Add Naga dependency with required features
3. Create `toys/toy5_naga_exploration/src/main.rs` skeleton

**Dependencies needed**:
```toml
[dependencies]
naga = { version = "27", features = ["wgsl-in", "msl-out", "validate"] }
```

**Basic structure**:
```rust
fn main() {
    println!("Toy 5: Naga Exploration");

    // Step 2: Parse test
    // Step 3: Validate test
    // Step 4: Translate test
    // Step 5: Benchmark test
    // Step 6: Error scenario tests
}
```

### Success Criteria

- [ ] `cargo build` succeeds in toy5 directory
- [ ] `cargo run` prints banner and exits cleanly
- [ ] Naga features available (`wgsl-in`, `msl-out`, `validate`)

---

## Step 2: Parse WGSL Shader

### Goal
Validate that Naga can parse real WGSL and produce IR

### Step 2.a: Write Parse Test

**Test strategy**:
- Use simple inline WGSL shader (fragment shader)
- Call `naga::front::wgsl::parse_str()`
- Assert module structure (has types, functions, entry points)

**Test outline**:
```rust
fn test_parse() {
    let wgsl = r#"
        @fragment
        fn main_fs() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let module = naga::front::wgsl::parse_str(wgsl)
        .expect("Parse should succeed");

    assert!(!module.entry_points.is_empty(), "Should have entry point");
    assert_eq!(module.entry_points[0].stage, naga::ShaderStage::Fragment);

    println!("✓ Parse test passed");
}
```

### Step 2.b: Implement Parse Function

**Tasks**:
1. Create `test_parse()` function in main.rs
2. Add inline WGSL shader
3. Call Naga parse API
4. Assert module properties
5. Call from `main()`

**Pattern**:
```rust
use naga::front::wgsl;

fn test_parse() {
    let wgsl = "...";
    match wgsl::parse_str(wgsl) {
        Ok(module) => {
            // Inspect and assert
        }
        Err(e) => panic!("Parse failed: {:?}", e),
    }
}
```

### Success Criteria

- [ ] Parse succeeds for valid WGSL
- [ ] Module has expected entry points
- [ ] Module structure inspectable (types, functions)
- [ ] Test prints success message

**Commit**: `feat(toy5): Step 2 - parse WGSL shader`

---

## Step 3: Validate Module

### Goal
Validate that Naga validation API works and produces ModuleInfo

### Step 3.a: Write Validation Test

**Test strategy**:
- Use module from Step 2
- Create Validator with different capability sets
- Assert validation succeeds and produces ModuleInfo
- Compare validation with `all()` vs `default()` capabilities

**Test outline**:
```rust
fn test_validate() {
    let module = /* from parse */;

    // Test 1: Validate with all capabilities
    let module_info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("Validation should succeed");

    assert!(!module_info.is_empty(), "Should have module info");

    // Test 2: Validate with default (conservative) capabilities
    let module_info2 = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .expect("Should work with default caps too");

    println!("✓ Validation test passed (all caps and default caps)");
}
```

### Step 3.b: Implement Validation Function

**Tasks**:
1. Create `test_validate()` function
2. Create Validator with `all()` capabilities
3. Call `validate(&module)`
4. Assert `ModuleInfo` returned
5. Repeat with `default()` capabilities
6. Call from `main()`

**Pattern**:
```rust
use naga::valid::{Validator, ValidationFlags, Capabilities};

fn test_validate(module: &naga::Module) {
    let mut validator = Validator::new(
        ValidationFlags::all(),
        Capabilities::all(),
    );

    match validator.validate(module) {
        Ok(info) => { /* success */ }
        Err(e) => panic!("Validation failed: {}", e),
    }
}
```

### Success Criteria

- [ ] Validation succeeds with `Capabilities::all()`
- [ ] Validation succeeds with `Capabilities::default()`
- [ ] ModuleInfo is non-empty
- [ ] Test prints success message

**Commit**: `feat(toy5): Step 3 - validate module with different capabilities`

---

## Step 4: Translate to MSL

### Goal
Validate that MSL backend produces readable output

### Step 4.a: Write Translation Test

**Test strategy**:
- Use module and ModuleInfo from previous steps
- Call MSL backend writer
- Assert MSL output is non-empty string
- Print MSL output for manual inspection

**Test outline**:
```rust
fn test_translate_msl() {
    let module = /* from parse */;
    let module_info = /* from validate */;

    let mut msl_source = String::new();
    naga::back::msl::Writer::new(&mut msl_source)
        .write(
            &module,
            &module_info,
            &naga::back::msl::Options {
                lang_version: (2, 4),
                ..Default::default()
            },
            &naga::back::msl::PipelineOptions::default(),
        )
        .expect("Translation should succeed");

    assert!(!msl_source.is_empty(), "MSL output should not be empty");
    assert!(msl_source.contains("fragment"), "Should have fragment function");

    println!("✓ Translation test passed");
    println!("\nMSL output:\n{}", msl_source);
}
```

### Step 4.b: Implement Translation Function

**Tasks**:
1. Create `test_translate_msl()` function
2. Create MSL Writer with Metal 2.4 options
3. Call `write()` with module and module_info
4. Assert output is valid
5. Print MSL for inspection
6. Call from `main()`

**Pattern**:
```rust
use naga::back::msl;

fn test_translate_msl(module: &naga::Module, info: &naga::valid::ModuleInfo) {
    let mut output = String::new();
    msl::Writer::new(&mut output)
        .write(module, info, &options, &pipeline_options)
        .expect("MSL write failed");

    println!("{}", output);
}
```

### Success Criteria

- [ ] Translation succeeds for valid module
- [ ] MSL output is non-empty
- [ ] MSL contains `fragment` keyword
- [ ] MSL is human-readable (manual inspection)
- [ ] Output printed to console

**Commit**: `feat(toy5): Step 4 - translate to MSL`

---

## Step 5: Benchmark Pipeline Performance

### Goal
Measure actual parse + validate + translate performance

### Step 5.a: Write Benchmark Test

**Test strategy**:
- Use `std::time::Instant` for timing
- Run pipeline 100 iterations
- Measure each stage separately
- Calculate averages
- Assert times are reasonable (< 5ms each stage)

**Test outline**:
```rust
fn test_benchmark() {
    let wgsl = "...";
    let iterations = 100;

    let mut parse_times = vec![];
    let mut validate_times = vec![];
    let mut translate_times = vec![];

    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let module = naga::front::wgsl::parse_str(wgsl).unwrap();
        parse_times.push(start.elapsed().as_micros());

        let start = std::time::Instant::now();
        let info = validator.validate(&module).unwrap();
        validate_times.push(start.elapsed().as_micros());

        let start = std::time::Instant::now();
        let mut msl = String::new();
        msl::Writer::new(&mut msl).write(&module, &info, &opts, &pipe_opts).unwrap();
        translate_times.push(start.elapsed().as_micros());
    }

    let avg_parse = parse_times.iter().sum::<u128>() / iterations as u128;
    let avg_validate = validate_times.iter().sum::<u128>() / iterations as u128;
    let avg_translate = translate_times.iter().sum::<u128>() / iterations as u128;

    println!("✓ Benchmark (100 iterations):");
    println!("  Parse:     {}μs", avg_parse);
    println!("  Validate:  {}μs", avg_validate);
    println!("  Translate: {}μs", avg_translate);
    println!("  Total:     {}μs", avg_parse + avg_validate + avg_translate);

    assert!(avg_parse < 5000, "Parse should be < 5ms");
    assert!(avg_validate < 5000, "Validate should be < 5ms");
    assert!(avg_translate < 5000, "Translate should be < 5ms");
}
```

### Step 5.b: Implement Benchmark Function

**Tasks**:
1. Create `test_benchmark()` function
2. Add timing for each pipeline stage
3. Calculate averages over iterations
4. Print results
5. Assert performance claims
6. Call from `main()`

**Pattern**:
```rust
use std::time::Instant;

fn benchmark_stage<F, R>(f: F, iterations: u32) -> u128
where F: Fn() -> R
{
    let mut times = vec![];
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = f();
        times.push(start.elapsed().as_micros());
    }
    times.iter().sum::<u128>() / iterations as u128
}
```

### Success Criteria

- [ ] Benchmark runs 100 iterations without error
- [ ] Parse time measured and printed
- [ ] Validate time measured and printed
- [ ] Translate time measured and printed
- [ ] Total pipeline time < 15ms (research claim validated)
- [ ] Results documented for reference update

**Commit**: `feat(toy5): Step 5 - benchmark pipeline performance`

---

## Step 6: Test Error Scenarios

### Goal
Validate error handling and inspect error messages

### Step 6.a: Write Error Tests

**Test strategy**:
- Test 1: Parse error (syntax error in WGSL)
- Test 2: Validation error (type mismatch)
- Test 3: Validation error (binding conflict)
- Assert errors include spans and useful messages

**Test outline**:
```rust
fn test_errors() {
    println!("\n--- Error Scenario Tests ---");

    // Test 1: Parse error
    let bad_wgsl = "@fragment fn main() { invalid syntax here }";
    match naga::front::wgsl::parse_str(bad_wgsl) {
        Err(e) => {
            println!("✓ Parse error caught: {:?}", e);
            // Could inspect e.spans() here
        }
        Ok(_) => panic!("Should have failed to parse"),
    }

    // Test 2: Type mismatch
    let type_error = r#"
        @fragment
        fn main() -> @location(0) vec4<f32> {
            return 1.0; // Wrong type
        }
    "#;
    let module = naga::front::wgsl::parse_str(type_error).unwrap();
    match validator.validate(&module) {
        Err(e) => {
            println!("✓ Validation error caught: {}", e);
            println!("  Error has {} span contexts", e.spans().count());
        }
        Ok(_) => panic!("Should have failed validation"),
    }

    // Test 3: Binding conflict
    let binding_error = r#"
        @group(0) @binding(0) var tex1: texture_2d<f32>;
        @group(0) @binding(0) var tex2: texture_2d<f32>;
        @fragment fn main() -> @location(0) vec4<f32> { return vec4(0.0); }
    "#;
    let module = naga::front::wgsl::parse_str(binding_error).unwrap();
    match validator.validate(&module) {
        Err(e) => println!("✓ Binding conflict caught: {}", e),
        Ok(_) => panic!("Should have failed validation"),
    }
}
```

### Step 6.b: Implement Error Test Function

**Tasks**:
1. Create `test_errors()` function
2. Add parse error test case
3. Add validation error test cases (type, binding)
4. Assert errors are caught
5. Print error messages and spans
6. Call from `main()`

**Pattern**:
```rust
fn test_error_scenario(wgsl: &str, expected_error: &str) {
    match parse_and_validate(wgsl) {
        Err(e) => {
            assert!(e.to_string().contains(expected_error));
            println!("✓ Expected error: {}", e);
        }
        Ok(_) => panic!("Should have failed"),
    }
}
```

### Success Criteria

- [ ] Parse errors are caught and inspectable
- [ ] Validation errors are caught with spans
- [ ] Type mismatch error is clear and actionable
- [ ] Binding conflict error is clear and actionable
- [ ] Error messages include context (not just error codes)
- [ ] Spans provide byte offsets (could map to source lines)

**Commit**: `feat(toy5): Step 6 - test error scenarios and inspect messages`

---

## Step 7: Test with Real Shader (Optional Stretch Goal)

### Goal
Validate findings with actual vibesurfer shader if time permits

### Step 7.a: Identify Real Shader

**Options**:
- Extract shader from toy4 (wireframe fragment shader)
- Or use embedded shader string from vibesurfer source

**Task**:
- Find shader source in `toys/toy4_spherical_chunks/src/`
- Copy into test or read from file

### Step 7.b: Run Full Pipeline on Real Shader

**Test strategy**:
- Parse real shader
- Validate with Metal capabilities
- Translate to MSL
- Inspect output
- Document any surprises

**Pattern**:
```rust
fn test_real_shader() {
    let wgsl = include_str!("../../toy4_spherical_chunks/src/shader.wgsl");
    // Or extract from Rust string literal

    let module = naga::front::wgsl::parse_str(wgsl)
        .expect("Real shader should parse");

    let info = validator.validate(&module)
        .expect("Real shader should validate");

    let mut msl = String::new();
    msl::Writer::new(&mut msl)
        .write(&module, &info, &opts, &pipe_opts)
        .expect("Real shader should translate");

    println!("✓ Real shader test passed");
    println!("MSL output length: {} chars", msl.len());
    // Could write to file for inspection
}
```

### Success Criteria

- [ ] Real shader parses successfully
- [ ] Real shader validates with appropriate capabilities
- [ ] Real shader translates to MSL
- [ ] MSL output inspected and documented
- [ ] Any surprises documented in LEARNINGS

**Commit**: `feat(toy5): Step 7 - test with real vibesurfer shader`

---

## Final Step: Document Findings

### Goal
Update reference documentation with practical learnings

### Tasks

1. Create `toys/toy5_naga_exploration/.ddd/LEARNINGS.md`
2. Document:
   - Performance measurements (actual times)
   - MSL output quality (readable? matches expectations?)
   - Error messages (useful? spans helpful?)
   - Capability constraints (did default() work or need all()?)
   - Any surprises or gotchas
3. Update `learnings/naga-reference.md`:
   - Add performance measurements
   - Update error handling section with examples
   - Add any practical gotchas discovered
   - Mark open questions as answered

### Success Criteria

- [ ] LEARNINGS.md created with findings
- [ ] Performance numbers documented
- [ ] Error patterns cataloged
- [ ] naga-reference.md updated with practical insights
- [ ] Open questions from research marked resolved or updated

**Commit**: `docs(toy5): document findings and update reference`

---

## Success Metrics

**Code quality**:
- [ ] Single file implementation (main.rs < 250 lines)
- [ ] All tests pass (assertions succeed)
- [ ] No panics on happy path
- [ ] Clean build with no warnings

**Research validation**:
- [ ] Validation is mandatory (confirmed - backends require ModuleInfo)
- [ ] Performance < 15ms total (measured and documented)
- [ ] Errors include spans (confirmed and examples shown)
- [ ] MSL output is readable (inspected)
- [ ] Capabilities matter (tested all vs default)

**Documentation**:
- [ ] LEARNINGS.md captures findings
- [ ] naga-reference.md updated
- [ ] Measurements included (parse/validate/translate times)
- [ ] Code examples from toy can be referenced

---

## Time Budget

- Step 1 (Setup): 5 min
- Step 2 (Parse): 10 min
- Step 3 (Validate): 15 min
- Step 4 (Translate): 15 min
- Step 5 (Benchmark): 20 min
- Step 6 (Errors): 20 min
- Step 7 (Real shader): 15 min (optional)
- Documentation: 30 min

**Total**: ~2 hours (within time box)

---

## Anti-Patterns to Avoid

- ❌ Building complete CLI tool (keep it simple)
- ❌ Testing all error scenarios (sample key patterns)
- ❌ Testing multiple backends (MSL only)
- ❌ Over-engineering test harness (inline tests OK)
- ❌ Perfect code (toy quality, not production)

---

This plan follows TDD discipline: write test → implement → verify → commit. Each step validates a specific claim from research. Final deliverable is updated reference documentation with practical findings.
