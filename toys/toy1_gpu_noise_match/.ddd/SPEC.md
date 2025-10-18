# Toy Model 1: GPU Noise Matching Specification

Validate that GPU compute shader produces visually equivalent noise output to CPU reference implementation.

---

## Overview

**What it does:** Samples 3D simplex noise on both CPU (current working implementation) and GPU (WGSL shader), then validates GPU produces hill-like terrain that visually matches CPU output.

**Key principles:**
- **Isolation**: Test only noise generation via rendered output comparison
- **Falsifiable**: Side-by-side visual rendering + optional numerical spot-check
- **Reference-driven**: CPU implementation is ground truth for visual appearance
- **Algorithm-agnostic**: GPU can use different simplex variant as long as output matches visually

**Scope:**
- Single-axis complexity: GPU noise implementation correctness (visual + basic numerical)
- Does NOT test: performance optimization, full ocean integration, audio reactivity

**Integration context:**
- **Input**: Seed value, sample positions (x, y, z), frequency/amplitude parameters
- **Output**: Rendered 2D heightmap showing terrain hills/valleys
- **Downstream**: If validated, GPU implementation replaces CPU in ocean mesh (fixes `gpu-compute-simplex` branch visual bug)

---

## Data Model

### Noise Sample

**Format**: Single 3D position → scalar noise value

```json
{
  "position": [x, y, z],
  "noise_value": 0.42,
  "source": "cpu" | "gpu"
}
```

**Fields**:
- `position`: [f32; 3] - 3D coordinates in noise field
- `noise_value`: f32 - Noise output, range [-1.0, 1.0]
- `source`: string - Whether computed on CPU or GPU

### Test Case

**Format**: Batch of positions with expected CPU results

```json
{
  "seed": 42,
  "samples": [
    {"position": [0.0, 0.0, 0.0], "expected": 0.0},
    {"position": [1.0, 2.0, 3.0], "expected": -0.234},
    {"position": [100.5, 200.7, 0.0], "expected": 0.567}
  ],
  "tolerance": 0.0001
}
```

**Fields**:
- `seed`: u32 - RNG seed for OpenSimplex initialization
- `samples`: array - Positions to test
- `expected`: f32 - CPU-computed reference value
- `tolerance`: f32 - Max absolute difference allowed (accounts for f32 precision)

---

## Core Operations

### Operation 1: Sample CPU Noise

**Syntax**: `sample_cpu(seed: u32, x: f32, y: f32, z: f32) -> f32`

**Parameters**:
- `seed`: Noise generator seed (required)
- `x, y, z`: 3D position coordinates (required)

**Example**:
```rust
let noise_gen = NoiseGenerator::new(42);
let value = noise_gen.sample_3d(1.0, 2.0, 3.0);
// value ≈ -0.234 (deterministic for given seed+position)
```

**Behavior**:
- Initializes `noise::OpenSimplex` with seed
- Calls `get([x, y, z])`
- Returns f32 in range [-1.0, 1.0]

**Validation**:
- Same seed + position → same output (deterministic)
- Output always in [-1.0, 1.0] range

---

### Operation 2: Sample GPU Noise

**Syntax**: `sample_gpu(seed: u32, positions: &[[f32; 3]]) -> Vec<f32>`

**Parameters**:
- `seed`: Noise generator seed (required)
- `positions`: Batch of 3D positions to sample (required)

**Example**:
```rust
let positions = vec![
    [0.0, 0.0, 0.0],
    [1.0, 2.0, 3.0],
];
let values = sample_gpu(42, &positions);
// values[0] ≈ 0.0
// values[1] ≈ -0.234
```

**Behavior**:
- Create compute shader with embedded seed (or pass as uniform)
- Upload positions to storage buffer
- Dispatch compute shader (one thread per position)
- Download noise results to CPU
- Return Vec<f32>

**Validation**:
- All outputs in [-1.0, 1.0] range
- Deterministic for same seed+position

---

### Operation 3: Compare Results

**Syntax**: `compare(cpu_values: &[f32], gpu_values: &[f32], tolerance: f32) -> Result<(), String>`

**Parameters**:
- `cpu_values`: Reference values from CPU
- `gpu_values`: Test values from GPU
- `tolerance`: Max absolute difference (default: 0.0001)

**Example**:
```rust
let cpu = vec![0.0, -0.234, 0.567];
let gpu = vec![0.00001, -0.23399, 0.56701];
let result = compare(&cpu, &gpu, 0.0001);
// result = Ok(()) - all within tolerance
```

**Behavior**:
- Check lengths match
- For each pair: `|cpu[i] - gpu[i]| <= tolerance`
- Return Ok if all match, Err with first mismatch details

**Validation**:
- Error message includes: index, cpu value, gpu value, difference

---

## Test Scenarios

### Simple: Origin Point
**Input**:
- Seed: 42
- Position: [0.0, 0.0, 0.0]

**Expected**:
- CPU and GPU return same value (within 0.0001)
- Value in range [-1.0, 1.0]

---

### Complex: Grid of Samples
**Input**:
- Seed: 12345
- Positions: 10×10×10 regular grid (1000 samples)
  - X: 0.0 to 9.0, step 1.0
  - Y: 0.0 to 9.0, step 1.0
  - Z: 0.0 to 9.0, step 1.0

**Expected**:
- All 1000 samples match CPU reference (within 0.0001)
- No NaN or Inf values
- Range check: all values in [-1.0, 1.0]

---

### Complex: Ocean-Scale Coordinates
**Input**:
- Seed: 42
- Positions matching ocean mesh usage:
  - Base terrain: `[x * 0.01, z * 0.01, 0.0]` for x,z in [0, 512]
  - Detail layer: `[x * 0.1, z * 0.1, time]` for time in [0.0, 10.0]

**Expected**:
- All samples match CPU (within 0.0001)
- Tests realistic frequency/coordinate ranges from actual ocean code

---

### Error: Seed Mismatch
**Input**:
- CPU seed: 42
- GPU seed: 43
- Same positions

**Expected**:
- Comparison fails (different seeds → different noise)
- Error message indicates mismatch

---

### Error: Out-of-Range Output
**Input**:
- Any valid position

**Expected**:
- If GPU returns value > 1.0 or < -1.0, test fails
- Indicates broken noise implementation

---

## Success Criteria

- [ ] Two runnable binaries exist: `toy1_cpu` and `toy1_gpu`
- [ ] Both binaries accept same CLI args: `--seed`, `--frequency`, `--size`
- [ ] `toy1_cpu` outputs `output_cpu.png` (grayscale heightmap)
- [ ] `toy1_gpu` outputs `output_gpu.png` (grayscale heightmap)
- [ ] CPU noise renders recognizable hill-like terrain (baseline)
- [ ] GPU compute shader compiles without errors
- [ ] GPU noise renders hill-like terrain (not flat, not random spikes, smooth gradients)
- [ ] Side-by-side visual comparison: CPU and GPU PNGs show similar terrain patterns
- [ ] Spot-check: Sample 10 positions, GPU values roughly match CPU (within ~10% relative error)
- [ ] Range validation: all GPU outputs in [-1.0, 1.0]
- [ ] Deterministic: Same seed+position → same GPU result across runs
- [ ] Ocean-scale parameters: Base terrain (freq=0.01) + Detail (freq=0.1) both work
- [ ] Execution time measured: GPU vs CPU for 1024×1024 samples (expect 10-30x GPU speedup)

---

## Implementation Notes

**GPU Simplex Source**:
- Current `gpu-compute-simplex` branch uses Stefan Gustavson's 3D simplex
- CPU uses `noise::OpenSimplex::get([x, y, z])`
- Both are simplex variants - **should produce similar smooth noise**
- **Problem**: Branch output looks wrong → likely bug in GPU implementation or parameter mismatch

**Debugging Strategy**:
1. Render CPU noise as 2D heightmap (ground truth visualization)
2. Render GPU noise with same parameters (side-by-side comparison)
3. If GPU looks wrong:
   - Check seed handling (GPU shader may not support seed parameter)
   - Check coordinate mapping (x,y,z order, scaling)
   - Check noise range (CPU returns [-1,1], verify GPU does too)
   - Validate simplex implementation (port CPU version if needed)

**Tolerance Rationale**:
- Visual equivalence > bit-exact match
- Spot-check: ~10% relative error acceptable (noise is approximate anyway)
- Main test: Does GPU terrain have smooth hills like CPU? (qualitative)

**Rendering Approach**:
- Simple 2D grid (e.g., 256×256)
- Sample noise at each point, render as grayscale heightmap
- CPU: PNG output using CPU sampling
- GPU: Compute shader writes to texture, copy to CPU, save PNG
- Compare PNGs visually

**Deliverable Binaries**:
- `toy1_cpu`: Renders CPU noise heightmap to `output_cpu.png`
- `toy1_gpu`: Renders GPU noise heightmap to `output_gpu.png`
- Both accept same CLI args: `--seed <u32> --frequency <f32> --size <usize>`
- User can run both, then visually compare PNG outputs side-by-side

**Non-Goals**:
- Exact numerical match (different algorithms fine)
- Performance optimization (measure, don't optimize yet)
- Full ocean integration (just noise validation)
