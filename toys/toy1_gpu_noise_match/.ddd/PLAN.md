# GPU Noise Matching - Implementation Plan

**Goal**: Build two standalone binaries that render CPU and GPU noise to PNG heightmaps for visual comparison, validating that GPU implementation produces hill-like terrain matching CPU reference.

**Scope**: Minimal proof-of-concept (single file per binary if possible, <300 lines each)

**Priority**: Debug visual mismatch in `gpu-compute-simplex` branch by isolating noise implementation

---

## Methodology

### TDD Approach
- **Test-first**: Write validation before implementation
- **Red → Green**: Failing test → minimal fix → passing test
- **Visual validation**: Primary test is PNG output inspection
- **Numerical spot-check**: Secondary validation (10 sample points)

### What to Test
- ✅ Noise range bounds ([-1, 1])
- ✅ Deterministic output (same seed → same result)
- ✅ Visual coherence (smooth gradients, hill-like patterns)
- ✅ Parameter scaling (frequency affects detail level)

### What NOT to Test
- ❌ Performance optimization (measure only)
- ❌ Exact numerical match between CPU/GPU
- ❌ Edge cases (NaN, Inf, extreme coordinates)
- ❌ Seed variation (just use one seed for validation)

---

## Step 1: CPU Reference Binary

### Goal
Create standalone binary that renders CPU noise (using `noise` crate) to PNG heightmap.

### Step 1.a: Write Tests

**Test strategy**:
- Integration test: Run binary, verify PNG created, check file size > 0
- Pixel validation: Load PNG, check all values in [0, 255] grayscale range
- Visual inspection: Manual check that output shows smooth hills (not random)

**Key test cases**:
1. **Basic execution**: `toy1_cpu --seed 42 --frequency 0.1 --size 256` produces `output_cpu.png`
2. **Deterministic**: Same args → same PNG (byte-identical)
3. **Range check**: Sample 10 pixels, verify grayscale values reasonable (not all black/white)

**Expected behavior**:
- Binary exits with code 0 on success
- PNG file exists after execution
- Grayscale heightmap shows terrain variation

### Step 1.b: Implement

**Tasks**:
1. Create `toys/toy1_gpu_noise_match/src/bin/toy1_cpu.rs`
2. Add CLI arg parsing (clap): `--seed`, `--frequency`, `--size`
3. Initialize `noise::OpenSimplex` with seed
4. Sample noise in 2D grid (z=0 for 2D slice)
5. Map noise [-1, 1] → grayscale [0, 255]
6. Write PNG using `image` crate
7. Print execution time

**Code pattern** (illustrative):
```rust
let simplex = OpenSimplex::new(seed);
for y in 0..size {
    for x in 0..size {
        let nx = x as f64 * frequency;
        let ny = y as f64 * frequency;
        let noise_val = simplex.get([nx, ny, 0.0]); // [-1, 1]
        let gray = ((noise_val + 1.0) * 127.5) as u8; // [0, 255]
        img.put_pixel(x, y, Luma([gray]));
    }
}
img.save("output_cpu.png")?;
```

**Error handling**:
- Invalid args → print usage, exit 1
- PNG write failure → print error, exit 1

### Success Criteria
- [ ] `toy1_cpu` binary compiles
- [ ] Accepts CLI args: `--seed`, `--frequency`, `--size`
- [ ] Produces `output_cpu.png` (256×256 grayscale)
- [ ] Visual inspection: PNG shows smooth hill-like terrain
- [ ] Deterministic: Same args → identical PNG
- [ ] Execution time printed (baseline for GPU comparison)
- [ ] Integration test passes

---

## Step 2: GPU Compute Binary (Simplified)

### Goal
Create standalone binary that renders GPU noise to PNG heightmap using compute shader.

### Step 2.a: Write Tests

**Test strategy**:
- Same as Step 1 (integration test + pixel validation)
- Compare GPU output to CPU output visually (manual inspection)
- Spot-check: Sample 10 positions, compare CPU vs GPU values (~10% tolerance)

**Key test cases**:
1. **Basic execution**: `toy1_gpu --seed 42 --frequency 0.1 --size 256` produces `output_gpu.png`
2. **Shader compilation**: No WGSL errors on startup
3. **Visual similarity**: GPU PNG shows similar terrain patterns to CPU PNG
4. **Spot-check**: Sample (x=128, y=128) and 9 other points, GPU values ≈ CPU values

**Expected behavior**:
- Compute shader compiles successfully
- GPU output shows smooth terrain (not flat, not spiky)
- Patterns roughly match CPU (hills in similar locations)

### Step 2.b: Implement

**Tasks**:
1. Create `toys/toy1_gpu_noise_match/src/bin/toy1_gpu.rs`
2. Add CLI arg parsing (same as CPU binary)
3. Initialize wgpu (device, queue)
4. Load compute shader from WGSL file
5. Create storage buffer for noise output
6. Pass seed/frequency as uniform
7. Dispatch compute shader (one thread per pixel)
8. Copy GPU buffer to CPU
9. Map to grayscale and write PNG
10. Print execution time

**Code pattern** (illustrative):
```rust
// Shader: src/bin/noise.wgsl
@group(0) @binding(0) var<storage, read_write> output: array<f32>;
@group(0) @binding(1) var<uniform> params: NoiseParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = f32(id.x) * params.frequency;
    let y = f32(id.y) * params.frequency;
    let noise_val = simplex3d(vec3<f32>(x, y, 0.0));
    let idx = id.y * params.size + id.x;
    output[idx] = noise_val;
}
```

**Rust side**:
- Create output buffer (size×size × f32)
- Create uniform buffer (NoiseParams struct)
- Dispatch: `(size/16, size/16, 1)` workgroups
- Copy buffer to CPU-readable staging buffer
- Map and convert to grayscale PNG

**Error handling**:
- Shader compilation failure → print WGSL error, exit 1
- GPU not available → print warning, exit 1
- Buffer mapping timeout → print error, exit 1

### Success Criteria
- [ ] `toy1_gpu` binary compiles
- [ ] Compute shader compiles without errors
- [ ] Produces `output_gpu.png` (256×256 grayscale)
- [ ] Visual inspection: PNG shows smooth terrain (not artifacts)
- [ ] Side-by-side comparison: GPU and CPU PNGs look similar
- [ ] Spot-check test: 10 positions match within ~10% relative error
- [ ] Execution time printed (expect 10-30x faster than CPU)
- [ ] Integration test passes

---

## Step 3: Simplex Noise Implementation (GPU)

### Goal
Port or validate 3D simplex noise implementation in WGSL to match CPU behavior.

### Step 3.a: Write Tests

**Test strategy**:
- Compare GPU simplex output to CPU simplex for same inputs
- Test known simplex properties (smoothness, range, determinism)

**Key test cases**:
1. **Origin point**: `simplex3d([0, 0, 0])` produces same value on CPU and GPU
2. **Grid sample**: 10×10×10 grid, all points match within tolerance
3. **Range validation**: All outputs in [-1, 1]
4. **Smoothness**: Adjacent points have small delta (< 0.5 typical)

**Expected behavior**:
- GPU simplex implementation is deterministic
- Output range matches CPU
- Visual terrain has similar smoothness

### Step 3.b: Implement

**Tasks**:
1. Review `gpu-compute-simplex` branch shader (Stefan Gustavson implementation)
2. Test if current implementation matches CPU (spot-check 10 points)
3. If mismatch:
   - Option A: Port `noise` crate's OpenSimplex to WGSL
   - Option B: Find alternative WGSL simplex that matches
   - Option C: Debug current implementation (check seed, coordinate mapping, scaling)
4. Add seed support to GPU shader (if missing)
5. Validate coordinate mapping (x,y,z order matches CPU)

**Code pattern** (illustrative):
```rust
// CPU reference
let cpu_val = OpenSimplex::new(seed).get([x, y, z]);

// GPU test
let gpu_val = run_gpu_simplex(seed, [x, y, z]);

assert!((cpu_val - gpu_val).abs() < 0.1,
    "Mismatch at [{x},{y},{z}]: CPU={cpu_val}, GPU={gpu_val}");
```

**Debugging steps if mismatch**:
1. Check seed: Does GPU shader accept seed parameter?
2. Check range: Are GPU outputs in [-1, 1]?
3. Check scaling: Is frequency applied correctly?
4. Check implementation: Does GPU simplex match known algorithm?

**Error handling**:
- Shader panic/crash → validate WGSL syntax
- Wrong range → check noise normalization
- Visual artifacts → inspect gradient calculation

### Success Criteria
- [ ] GPU simplex produces values in [-1, 1] range
- [ ] Origin point test passes (CPU ≈ GPU at [0,0,0])
- [ ] Grid test passes (10×10×10 samples match within ~10%)
- [ ] Seed parameter works (different seeds → different noise)
- [ ] Visual output shows smooth gradients (no grid artifacts)
- [ ] Integration with toy binaries works (same seed produces similar terrains)

---

## Step 4: Validation and Documentation

### Goal
Validate that toy meets SPEC success criteria and document findings.

### Step 4.a: Write Tests

**Test strategy**:
- Run both binaries with identical args
- Visual comparison (human inspection)
- Automated spot-check (10 positions)
- Performance measurement

**Key test cases**:
1. **Ocean-scale base terrain**: `--frequency 0.01 --seed 42`
2. **Ocean-scale detail layer**: `--frequency 0.1 --seed 42`
3. **Performance test**: 1024×1024 grid, measure CPU vs GPU time

**Expected behavior**:
- CPU and GPU produce visually similar terrain
- GPU is 10-30x faster than CPU
- Both are deterministic (same args → same output)

### Step 4.b: Implement

**Tasks**:
1. Run `toy1_cpu --seed 42 --frequency 0.01 --size 512` (base terrain)
2. Run `toy1_gpu --seed 42 --frequency 0.01 --size 512`
3. Visual comparison: Open both PNGs side-by-side
4. If mismatch: Debug (check Step 3 implementation)
5. Repeat for detail layer (`--frequency 0.1`)
6. Document findings in `LEARNINGS.md`:
   - What worked?
   - What didn't match?
   - Performance comparison
   - Next steps for integration

**Code pattern** (illustrative):
```bash
# Generate CPU reference
cargo run --bin toy1_cpu -- --seed 42 --frequency 0.01 --size 512
# Output: output_cpu.png (1.2s)

# Generate GPU version
cargo run --bin toy1_gpu -- --seed 42 --frequency 0.01 --size 512
# Output: output_gpu.png (0.05s) - 24x faster

# Visual comparison
open output_cpu.png output_gpu.png
# Inspect: Do hills match? Are patterns similar?
```

**Error handling**:
- Visual mismatch → document in LEARNINGS, revisit Step 3
- Performance worse than expected → note in LEARNINGS, not blocking
- Determinism failure → critical bug, must fix

### Success Criteria
- [ ] Base terrain test (freq=0.01): CPU and GPU visually similar
- [ ] Detail layer test (freq=0.1): CPU and GPU visually similar
- [ ] Performance: GPU 10x+ faster than CPU for 1024×1024 grid
- [ ] Deterministic: Re-run produces identical PNGs
- [ ] LEARNINGS.md documents findings (what worked, what didn't, next steps)
- [ ] README.md added with usage instructions
- [ ] All SPEC success criteria met

---

## Commit Discipline

After each step completion:
```bash
git add toys/toy1_gpu_noise_match/
git commit -m "feat(toy1): complete Step N - <description>"
```

Example commits:
- `feat(toy1): complete Step 1 - CPU reference binary`
- `test(toy1): add integration tests for CPU binary`
- `feat(toy1): complete Step 2 - GPU compute binary`
- `fix(toy1): correct simplex noise range normalization`
- `docs(toy1): add LEARNINGS and README`

---

## Time Budget

**Total estimate**: 2-3 hours
- Step 1 (CPU binary): 30 min
- Step 2 (GPU binary): 45 min
- Step 3 (Simplex debug): 45-90 min (depends on issue complexity)
- Step 4 (Validation): 30 min

**Exit conditions**:
- ✅ Success: GPU and CPU visually match → integrate into main ocean mesh
- ⚠️ Partial: GPU works but doesn't match → document gap, consider alternative
- ❌ Failure: GPU fundamentally broken → may need different approach

---

## Dependencies

**Crates needed**:
- `noise` - CPU simplex reference (OpenSimplex)
- `image` - PNG encoding/decoding
- `wgpu` - GPU compute
- `bytemuck` - Buffer casting
- `pollster` - Async runtime (for wgpu init)
- `clap` - CLI parsing

**Files created**:
- `toys/toy1_gpu_noise_match/Cargo.toml`
- `toys/toy1_gpu_noise_match/src/bin/toy1_cpu.rs`
- `toys/toy1_gpu_noise_match/src/bin/toy1_gpu.rs`
- `toys/toy1_gpu_noise_match/src/noise.wgsl` (GPU simplex shader)
- `toys/toy1_gpu_noise_match/.ddd/LEARNINGS.md` (findings)
- `toys/toy1_gpu_noise_match/README.md` (usage)

---

## Success Definition

**Primary**: GPU and CPU produce visually similar terrain (smooth hills, similar patterns)

**Secondary**: Understand why `gpu-compute-simplex` branch doesn't match, document fix

**Deliverable**: Two working binaries + documentation enabling main integration
