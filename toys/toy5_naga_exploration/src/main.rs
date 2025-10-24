use naga::front::wgsl;

fn main() {
    println!("Toy 5: Naga Exploration");
    println!("======================\n");

    // Step 2: Parse test
    test_parse();

    // Step 3: Validate test
    // Step 4: Translate test
    // Step 5: Benchmark test
    // Step 6: Error scenario tests
}

fn test_parse() {
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
}
