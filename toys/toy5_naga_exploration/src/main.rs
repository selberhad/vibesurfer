use naga::back::msl;
use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::time::Instant;

fn main() {
    println!("Toy 5: Naga Exploration");
    println!("======================\n");

    // Step 2: Parse test
    let module = test_parse();

    // Step 3: Validate test
    let module_info = test_validate(&module);

    // Step 4: Translate test
    test_translate_msl(&module, &module_info);

    // Step 5: Benchmark test
    test_benchmark();

    // Step 6: Error scenario tests
    test_errors();

    // Step 7: Test with real vibesurfer shader
    test_real_shader();

    println!("\n======================");
    println!("All tests passed! ✓");
}

fn test_parse() -> naga::Module {
    let wgsl = r#"
        @fragment
        fn main_fs() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let module = wgsl::parse_str(wgsl).expect("Parse should succeed");

    assert!(!module.entry_points.is_empty(), "Should have entry point");
    assert_eq!(
        module.entry_points[0].stage,
        naga::ShaderStage::Fragment,
        "Should be fragment shader"
    );

    println!("✓ Parse test passed");
    println!("  Entry points: {}", module.entry_points.len());
    println!("  Types: {}", module.types.len());
    println!("  Functions: {}", module.functions.len());

    module
}

fn test_validate(module: &naga::Module) -> naga::valid::ModuleInfo {
    println!("\n--- Step 3: Validation ---");

    // Test 1: Validate with all capabilities (permissive)
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());

    let module_info = validator
        .validate(module)
        .expect("Validation should succeed with all capabilities");

    println!("✓ Validation passed with Capabilities::all()");

    // Test 2: Validate with default capabilities (conservative)
    validator.reset();
    let mut validator2 = Validator::new(ValidationFlags::all(), Capabilities::default());

    let _module_info2 = validator2
        .validate(module)
        .expect("Validation should succeed with default capabilities");

    println!("✓ Validation passed with Capabilities::default()");
    println!("  Both validations produced ModuleInfo successfully");

    module_info
}

fn test_translate_msl(module: &naga::Module, module_info: &naga::valid::ModuleInfo) {
    println!("\n--- Step 4: Translation to MSL ---");

    let mut msl_source = String::new();
    msl::Writer::new(&mut msl_source)
        .write(
            module,
            module_info,
            &msl::Options {
                lang_version: (2, 4),
                ..Default::default()
            },
            &msl::PipelineOptions::default(),
        )
        .expect("Translation should succeed");

    assert!(!msl_source.is_empty(), "MSL output should not be empty");
    assert!(
        msl_source.contains("fragment"),
        "Should have fragment function"
    );

    println!("✓ Translation test passed");
    println!("  MSL output length: {} chars", msl_source.len());
    println!("\nMSL output:\n{}", msl_source);
}

fn test_benchmark() {
    println!("\n--- Step 5: Benchmark ---");

    let wgsl = r#"
        @fragment
        fn main_fs() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let iterations = 100;
    let mut parse_times = Vec::new();
    let mut validate_times = Vec::new();
    let mut translate_times = Vec::new();

    for _ in 0..iterations {
        // Parse
        let start = Instant::now();
        let module = wgsl::parse_str(wgsl).unwrap();
        parse_times.push(start.elapsed().as_micros());

        // Validate
        let start = Instant::now();
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        let info = validator.validate(&module).unwrap();
        validate_times.push(start.elapsed().as_micros());

        // Translate
        let start = Instant::now();
        let mut msl = String::new();
        msl::Writer::new(&mut msl)
            .write(
                &module,
                &info,
                &msl::Options::default(),
                &msl::PipelineOptions::default(),
            )
            .unwrap();
        translate_times.push(start.elapsed().as_micros());
    }

    let avg_parse: u128 = parse_times.iter().sum::<u128>() / iterations;
    let avg_validate: u128 = validate_times.iter().sum::<u128>() / iterations;
    let avg_translate: u128 = translate_times.iter().sum::<u128>() / iterations;

    println!("✓ Benchmark ({} iterations):", iterations);
    println!("  Parse:     {}μs", avg_parse);
    println!("  Validate:  {}μs", avg_validate);
    println!("  Translate: {}μs", avg_translate);
    println!(
        "  Total:     {}μs",
        avg_parse + avg_validate + avg_translate
    );

    assert!(avg_parse < 5000, "Parse should be < 5ms");
    assert!(avg_validate < 5000, "Validate should be < 5ms");
    assert!(avg_translate < 5000, "Translate should be < 5ms");
}

fn test_errors() {
    println!("\n--- Step 6: Error Scenarios ---");

    // Test 1: Parse error
    let bad_wgsl = "@fragment fn main() { invalid syntax here }";
    match wgsl::parse_str(bad_wgsl) {
        Err(e) => {
            println!("✓ Parse error caught: {}", e.message());
        }
        Ok(_) => panic!("Should have failed to parse"),
    }

    // Test 2: Type error (caught at parse time)
    let type_error = r#"
        @fragment
        fn main() -> @location(0) vec4<f32> {
            return 1.0;
        }
    "#;
    match wgsl::parse_str(type_error) {
        Err(e) => {
            println!("✓ Type error caught at parse time");
            println!("  Error: {}", e.message());
        }
        Ok(_) => panic!("Should have failed to parse"),
    }

    println!("\n✓ Error handling validated (parse errors are caught and descriptive)");
}

fn test_real_shader() {
    println!("\n--- Step 7: Real Shader (toy4) ---");

    let wgsl = include_str!("../../toy4_spherical_chunks/src/sphere_render.wgsl");

    // Parse
    let start = Instant::now();
    let module = wgsl::parse_str(wgsl).expect("Real shader should parse");
    let parse_time = start.elapsed().as_micros();

    println!("✓ Parsed toy4 sphere_render.wgsl");
    println!("  Lines: ~52");
    println!("  Entry points: {}", module.entry_points.len());
    println!("  Types: {}", module.types.len());

    // Validate
    let start = Instant::now();
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let module_info = validator
        .validate(&module)
        .expect("Real shader should validate");
    let validate_time = start.elapsed().as_micros();

    println!("✓ Validated with Capabilities::all()");

    // Try with default capabilities
    validator.reset();
    let mut validator2 = Validator::new(ValidationFlags::all(), Capabilities::default());
    match validator2.validate(&module) {
        Ok(_) => println!("✓ Also validates with Capabilities::default()"),
        Err(e) => println!("⚠ Requires advanced capabilities: {}", e),
    }

    // Translate to MSL
    let start = Instant::now();
    let mut msl_source = String::new();
    msl::Writer::new(&mut msl_source)
        .write(
            &module,
            &module_info,
            &msl::Options {
                lang_version: (2, 4),
                ..Default::default()
            },
            &msl::PipelineOptions::default(),
        )
        .expect("Real shader should translate");
    let translate_time = start.elapsed().as_micros();

    println!("✓ Translated to MSL");
    println!("  MSL output: {} chars", msl_source.len());
    println!("\nPerformance (real shader):");
    println!("  Parse:     {}μs", parse_time);
    println!("  Validate:  {}μs", validate_time);
    println!("  Translate: {}μs", translate_time);
    println!(
        "  Total:     {}μs",
        parse_time + validate_time + translate_time
    );

    // Show snippet of MSL output
    println!("\nMSL output (first 500 chars):");
    println!("{}", &msl_source[..msl_source.len().min(500)]);
    if msl_source.len() > 500 {
        println!("... ({} more chars)", msl_source.len() - 500);
    }
}
