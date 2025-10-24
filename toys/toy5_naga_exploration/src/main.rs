use naga::front::wgsl;
use naga::valid::{Capabilities, ValidationFlags, Validator};

fn main() {
    println!("Toy 5: Naga Exploration");
    println!("======================\n");

    // Step 2: Parse test
    let module = test_parse();

    // Step 3: Validate test
    test_validate(&module);

    // Step 4: Translate test
    // Step 5: Benchmark test
    // Step 6: Error scenario tests
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

fn test_validate(module: &naga::Module) {
    println!("\n--- Step 3: Validation ---");

    // Test 1: Validate with all capabilities (permissive)
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());

    let _module_info = validator
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
}
