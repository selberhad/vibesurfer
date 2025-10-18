# Toy Model 3: Toroidal Camera Navigation Specification

Validate infinite 360° camera navigation on a toroidal terrain mesh with coherent noise sampling.

## Overview

**What it does:** Implements a camera that can move freely in any direction across a toroidal (wrapping) terrain surface. The terrain grid wraps seamlessly at boundaries, allowing infinite navigation while maintaining terrain coherence (same world position = same height).

**Key principles:**
- Camera moves freely in world space (any velocity vector, including zero)
- Terrain grid has fixed topology (512×512 vertices) on a torus
- Vertex positions computed relative to camera (wrapped for nearest distance)
- Noise sampled at unwrapped world coordinates for coherence
- No per-frame grid repositioning - wrapping handled by modulo math

**Scope:** Isolates two complexity axes:
1. Toroidal coordinate wrapping (camera-relative vertex positioning)
2. World-space noise coherence (same world XZ = same height regardless of torus wrapping)

**Integration context:**
- Input: Camera position (world-space XZ), camera velocity (for future player controls)
- Output: Seamlessly wrapping terrain that supports 360° navigation
- Foundation for main vibesurfer gameplay (surfing, carving, diving maneuvers)

## Data Model

### CameraState (CPU-side)

```rust
struct CameraState {
    position: Vec3,      // World-space position (unbounded, e.g., x=5000.0, z=12000.0)
    velocity: Vec3,      // Current velocity vector (e.g., [10.0, 0.0, 5.0] m/s)
    altitude: f32,       // Height above terrain (e.g., 80.0m)
}
```

### TorusParams (Uniform Buffer)

```wgsl
struct TorusParams {
    torus_extent_x: f32,     // Torus wrap distance X (e.g., 1024.0m)
    torus_extent_z: f32,     // Torus wrap distance Z (e.g., 1024.0m)
    camera_world_x: f32,     // Camera position (unwrapped world space)
    camera_world_z: f32,     // Camera position (unwrapped world space)
}
```

### TerrainParams (Uniform Buffer, extended from toy2)

```wgsl
struct TerrainParams {
    base_amplitude: f32,
    base_frequency: f32,
    detail_amplitude: f32,
    detail_frequency: f32,
    camera_world_x: f32,     // Unwrapped world position
    camera_world_z: f32,
    grid_size: u32,
    grid_spacing: f32,
    time: f32,
    torus_extent_x: f32,     // For wrapping calculations
    torus_extent_z: f32,
}
```

### Vertex (Storage Buffer, same as toy2)

```wgsl
struct Vertex {
    position: vec3<f32>,  // Camera-relative position (wrapped)
    _padding1: f32,
    uv: vec2<f32>,
    _padding2: vec2<f32>,
}
```

## Core Operations

### Operation 1: Update Camera Position

**Syntax:**
```rust
camera.update(delta_time: f32, input_velocity: Vec3)
```

**Parameters:**
- `delta_time: f32` - Time since last frame (seconds)
- `input_velocity: Vec3` - Desired velocity vector (m/s, any direction including zero)

**Behavior:**
1. Update velocity: `camera.velocity = input_velocity`
2. Update position: `camera.position += camera.velocity * delta_time`
3. Position is unbounded (no wrapping on CPU - can be x=10000.0, z=50000.0)

**Validation:**
- Delta time > 0
- Velocity magnitude reasonable (e.g., < 1000 m/s to prevent huge jumps)

**Example:**
```rust
// Move forward at 10 m/s
camera.update(0.016, Vec3::new(0.0, 0.0, 10.0));

// Move diagonally northeast at 15 m/s
camera.update(0.016, Vec3::new(10.6, 0.0, 10.6));

// Stop (velocity = 0)
camera.update(0.016, Vec3::ZERO);
```

### Operation 2: Compute Toroidal Vertex Positions (GPU)

**Syntax:**
```wgsl
@compute @workgroup_size(256)
fn compute_terrain(global_id: vec3<u32>)
```

**Parameters (via uniform buffer):**
- `camera_world_x, camera_world_z` - Unwrapped camera position
- `torus_extent_x, torus_extent_z` - Torus dimensions
- `grid_size, grid_spacing` - Grid topology

**Behavior:**
1. Calculate vertex position in torus local space:
   ```wgsl
   let local_x = f32(x) * grid_spacing;  // 0 to torus_extent_x
   let local_z = f32(z) * grid_spacing;  // 0 to torus_extent_z
   ```

2. Wrap camera position to torus space:
   ```wgsl
   let camera_torus_x = camera_world_x % torus_extent_x;
   let camera_torus_z = camera_world_z % torus_extent_z;
   ```

3. Calculate camera-relative offset (nearest distance on torus):
   ```wgsl
   var dx = local_x - camera_torus_x;
   var dz = local_z - camera_torus_z;

   // Wrap to nearest (handle torus seam)
   if (dx > torus_extent_x * 0.5) { dx -= torus_extent_x; }
   if (dx < -torus_extent_x * 0.5) { dx += torus_extent_x; }
   if (dz > torus_extent_z * 0.5) { dz -= torus_extent_z; }
   if (dz < -torus_extent_z * 0.5) { dz += torus_extent_z; }
   ```

4. Sample noise at unwrapped world coordinates:
   ```wgsl
   let world_noise_x = camera_world_x + dx;
   let world_noise_z = camera_world_z + dz;
   let height = simplex3d(vec3(world_noise_x * freq, world_noise_z * freq, time)) * amp;
   ```

5. Write camera-relative position:
   ```wgsl
   vertices[idx].position = vec3(dx, height, dz);
   ```

**Validation:**
- Wrapping logic preserves continuity (no seams at torus boundaries)
- Same world coordinates always produce same height (noise coherence)
- Vertex positions stay within camera view frustum (wrapping keeps grid centered on camera)

**Example:**
```wgsl
// Camera at world position (2000.0, 500.0) on 1024m torus
// camera_torus = (2000.0 % 1024.0, 500.0 % 1024.0) = (976.0, 500.0)

// Vertex at local_pos = (10.0, 510.0)
// dx = 10.0 - 976.0 = -966.0
// dz = 510.0 - 500.0 = 10.0

// Wrap dx (< -512.0, so add torus_extent)
// dx = -966.0 + 1024.0 = 58.0

// Camera-relative position: (58.0, height, 10.0)
// This vertex appears 58m in front of camera (wrapped correctly)
```

### Operation 3: Handle Camera Stop

**Syntax:**
```rust
camera.update(delta_time, Vec3::ZERO)
```

**Behavior:**
1. Velocity becomes zero
2. Camera position unchanged
3. Terrain vertices stay at same camera-relative positions
4. Noise sampling uses same world coordinates → terrain static

**Validation:**
- Terrain does not move when velocity = 0
- Heights remain consistent across frames
- No visual "drift" or instability

## Test Scenarios

### Simple: Static Camera (Zero Velocity)

**Setup:**
- Camera at world position (0.0, 80.0, 0.0)
- Velocity = Vec3::ZERO
- Torus extent = 1024m
- Grid = 512×512, 2m spacing

**Expected:**
- Terrain renders centered on camera
- Heights deterministic (same seed = same terrain)
- No movement across frames

**Success criteria:**
- [ ] Terrain visible and centered
- [ ] Same frame renders identical across multiple runs (deterministic noise)
- [ ] FPS stable (>60)

### Complex: Continuous Forward Motion

**Setup:**
- Camera starts at (0.0, 80.0, 0.0)
- Velocity = (0.0, 0.0, 10.0) - forward at 10 m/s
- Run for 120 seconds (camera travels 1200m, wraps torus once)

**Expected:**
- Terrain scrolls smoothly backward
- No visible seam when crossing torus boundary (at z=1024m)
- Terrain coherence: same world position has same height before/after wrap
- FPS ≥60 throughout

**Success criteria:**
- [ ] Smooth motion (no stuttering)
- [ ] No seam visible at torus wrap boundary
- [ ] Terrain height coherent (record heights at z=0, z=1024, z=2048 - should match)
- [ ] FPS ≥60 for full 120 seconds

### Complex: Diagonal Motion

**Setup:**
- Camera starts at (0.0, 80.0, 0.0)
- Velocity = (7.07, 0.0, 7.07) - northeast at ~10 m/s diagonal
- Run for 60 seconds

**Expected:**
- Terrain scrolls diagonally (southwest direction relative to camera)
- Wrapping works in both X and Z dimensions
- No visual artifacts at corner wraps

**Success criteria:**
- [ ] Diagonal motion smooth
- [ ] Wrapping works independently for X and Z
- [ ] No artifacts when both X and Z wrap simultaneously
- [ ] FPS ≥60

### Complex: Stop and Resume

**Setup:**
1. Camera moves forward at 10 m/s for 10 seconds (reaches z=100m)
2. Stop (velocity = 0) for 5 seconds
3. Resume forward at 10 m/s for 10 seconds

**Expected:**
- During motion: terrain scrolls
- During stop: terrain static (no drift)
- Resume: terrain resumes scrolling smoothly

**Success criteria:**
- [ ] Terrain stops moving when velocity = 0
- [ ] No visual "pop" or discontinuity when stopping
- [ ] No visual "pop" when resuming motion
- [ ] Same terrain heights before/during/after stop

### Complex: 360° Circle

**Setup:**
- Camera flies in circle (radius = 200m, period = 60s)
- Velocity tangent to circle, magnitude = 2πr/T ≈ 21 m/s
- Camera position: `(200*cos(θ), 80, 200*sin(θ))` where θ = 2πt/60

**Expected:**
- Camera orbits center point
- Terrain rotates around camera (opposite direction)
- Wrapping works at all angles
- After 60s, returns to start with same terrain visible

**Success criteria:**
- [ ] Smooth circular motion
- [ ] Terrain rotates continuously around camera
- [ ] Same terrain visible after completing circle (coherence)
- [ ] FPS ≥60 throughout orbit

### Error: Large Time Step

**Setup:**
- Camera velocity = 10 m/s
- delta_time = 10.0 seconds (huge jump, simulates lag spike)

**Expected:**
- Camera jumps 100m forward
- Terrain updates correctly (no visual artifacts from discontinuity)
- Wrapping still works (camera may cross torus boundary)

**Success criteria:**
- [ ] No crash or GPU errors
- [ ] Terrain renders correctly after large jump
- [ ] Wrapping math handles large position changes

## Success Criteria

**Functional Requirements:**
- [ ] Camera can move in any direction (360° navigation)
- [ ] Camera can stop (velocity = 0) without terrain drift
- [ ] Terrain wraps seamlessly at torus boundaries (no visible seams)
- [ ] Noise coherence: same world XZ always produces same height
- [ ] Vertex positions computed correctly (camera-relative with wrapping)

**Performance Requirements:**
- [ ] FPS ≥60 at 512×512 grid during motion
- [ ] No performance degradation when crossing torus boundaries
- [ ] Camera position updates have negligible CPU cost

**Visual Validation:**
- [ ] Terrain appears to scroll smoothly during motion
- [ ] No seams, pops, or discontinuities at torus boundaries
- [ ] Terrain static when camera stopped
- [ ] Wrapping works in all directions (forward, backward, left, right, diagonal)

**Coherence Validation:**
- [ ] Record terrain heights at world position (X, Z) at t=0
- [ ] Move camera, return to same world position at t=60
- [ ] Heights match (deterministic, coherent noise)

**Code Quality:**
- [ ] Wrapping logic clearly documented in shader
- [ ] No modulo edge cases (handle negative positions correctly)
- [ ] Camera position unbounded (no overflow for reasonable gameplay durations)

## Out of Scope (Deferred to Main Integration)

- ❌ Player input controls (keyboard/gamepad)
- ❌ Camera altitude adjustment (terrain following)
- ❌ Banking/tilting during turns
- ❌ Audio reactivity (focus on navigation only)
- ❌ Collision detection
- ❌ Recording/capture (manual testing sufficient)
