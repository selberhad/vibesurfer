# Toy3 Mathematical Review: Toroidal Terrain Navigation

## Problem Statement

We want to create an **infinite terrain** that allows a camera to navigate in any direction (360°) without visible boundaries, similar to flying a camera around a bumpy donut (torus) in 3ds Max.

## Current Implementation Analysis

### What We Have Now

**Grid Structure:**
- 256×256 vertices (configurable)
- 2m spacing between vertices
- Total extent: 512m × 512m

**Compute Shader (terrain_compute.wgsl):**
```wgsl
// Lines 132-154: Calculate vertex positions
let local_x = f32(x) * params.grid_spacing;  // 0 to 512m
let local_z = f32(z) * params.grid_spacing;  // 0 to 512m

// Wrap camera position to torus space
let camera_torus_x = camera_pos.x - floor(camera_pos.x / torus_extent_x) * torus_extent_x;
let camera_torus_z = camera_pos.z - floor(camera_pos.z / torus_extent_z) * torus_extent_z;

// Calculate offset from camera
var dx = local_x - camera_torus_x;
var dz = local_z - camera_torus_z;

// Wrap to nearest distance (THIS IS WHERE THE BUG IS)
if (dx > half_extent_x) { dx -= torus_extent_x; }
if (dx < -half_extent_x) { dx += torus_extent_x; }
if (dz > half_extent_z) { dz -= torus_extent_z; }
if (dz < -half_extent_z) { dz += torus_extent_z; }

// Sample noise at world coordinates
let world_x = camera_pos.x + dx;
let world_z = camera_pos.z + dz;
let height = sample_noise(world_x, world_z);

// LATEST CHANGE: Write fixed world positions (not camera-relative)
vertices[idx].position = vec3(local_x, height, local_z);
```

**Camera (lib.rs:128-144):**
```rust
// BROKEN: We accept camera_pos but don't use it!
pub fn create_perspective_view_proj_matrix(camera_pos: [f32; 3], aspect: f32) {
    let eye = Vec3::new(0.0, 80.0, 0.0);  // HARDCODED!
    let target = Vec3::new(0.0, 20.0, 300.0);  // HARDCODED!
    // ...
}
```

### The Core Problem

**We're doing a hybrid approach that doesn't work:**

1. ✅ Vertices are written at fixed world positions (0-512m) - CORRECT
2. ❌ Camera is STILL at origin (not using camera_pos parameter) - WRONG
3. ⚠️ Noise sampling uses wrapped coordinates - QUESTIONABLE
4. ❌ Wrapping logic tries to make vertices "camera-relative" but then writes world positions - CONTRADICTORY

**The oscillation is caused by:**
- We write vertices at fixed positions (local_x, local_z)
- But we're STILL calculating dx/dz wrapping (which does nothing now)
- Camera is stuck at origin looking at a fixed 512m patch
- As camera moves in world space (via velocity), it just flies away from the terrain!

---

## The 3ds Max Model (What We Want)

### How a Torus Works in 3ds Max

**Mesh:**
- Vertices at FIXED world positions arranged in a torus topology
- Example: 256 vertices around major radius, 256 around minor radius
- Positions: `[x, y, z] = [R*cos(u), r*sin(v), R*sin(u) + r*cos(v)]`
- Where u, v ∈ [0, 2π] are torus coordinates

**Camera:**
- Orbits the torus in world space
- View matrix transforms world positions to camera space
- Projection matrix creates perspective

**Key insight:**
- The mesh NEVER CHANGES
- Only the view matrix updates as camera moves
- Wrapping is IMPLICIT in the torus topology (u and v wrap at 2π)

### Simplified Flat Torus (What We Need)

Since we want a **flat** terrain (not a geometric donut), we use a different torus parameterization:

**Flat Torus (aka 2-torus):**
- It's topologically a torus (surface wraps in both X and Z)
- But geometrically it's a flat plane with periodic boundary conditions
- Vertices arranged in a regular grid with wraparound connections

**Vertex Positions:**
```
For grid indices (i, j) where i, j ∈ [0, N-1]:
  x = i * spacing  // 0 to (N-1)*spacing
  z = j * spacing  // 0 to (N-1)*spacing
  y = noise(x, z)  // Height from noise function
```

**Topology (Index Buffer):**
- Each quad (i, j) connects to (i+1 mod N, j+1 mod N)
- This makes edges wrap around (creates the torus topology)

**Key: The grid coordinates wrap, not the vertex positions!**

---

## What's Wrong With Our Current Approach

### Problem 1: Camera Not Moving in World Space

**Current code:**
```rust
let eye = Vec3::new(0.0, 80.0, 0.0);  // ALWAYS at origin!
```

**Should be:**
```rust
// Wrap camera position to torus world space
let torus_x = camera_pos[0] % torus_extent_x;
let torus_z = camera_pos[2] % torus_extent_z;
let eye = Vec3::new(torus_x, camera_pos[1] + 80.0, torus_z);
```

### Problem 2: Noise Sampling Uses Wrapped Coordinates

**Current:**
```wgsl
let world_x = camera_pos.x + dx;  // dx is wrapped
```

**Problem:**
- `dx` has been wrapped to nearest distance
- This creates discontinuities in noise when wrapping occurs
- We want CONTINUOUS noise across the infinite plane

**Should be:**
```wgsl
// Sample noise at vertex's ACTUAL world position
// NOT wrapped to torus extent
let world_x = camera_pos.x + (local_x - camera_torus_x);
let world_z = camera_pos.z + (local_z - camera_torus_z);
```

Wait, this is still wrong...

### Problem 3: Fundamental Confusion About Coordinate Spaces

We're mixing three different coordinate systems:

1. **Grid Space:** Vertex indices (0 to N-1)
2. **Torus Space:** World positions on the torus (0 to extent)
3. **Infinite Plane Space:** Unwrapped world positions (-∞ to +∞)

**For noise coherence, we need:** Same position in infinite plane space = same height

**For rendering, we need:** Vertices in torus space, camera in torus space

**The disconnect:**
- We're trying to sample noise in infinite plane space
- But render geometry in torus space
- These need to be DECOUPLED

---

## The Correct Solution: Separate Concerns

### Approach 1: Pure Torus (Simplest)

**Stop trying to maintain "infinite plane" coherence. Just treat it as a torus.**

```wgsl
// Vertex positions in torus space (0 to extent)
let torus_x = f32(x) * spacing;
let torus_z = f32(z) * spacing;

// Sample noise at torus positions (wraps naturally)
let noise_x = torus_x;
let noise_z = torus_z;
let height = sample_noise(noise_x, noise_z);

// Write torus-space position
vertices[idx].position = vec3(torus_x, height, torus_z);
```

```rust
// Camera orbits torus in torus space
let torus_x = camera_pos[0] % torus_extent;
let torus_z = camera_pos[2] % torus_extent;
let eye = Vec3::new(torus_x, 80.0, torus_z);
let target = Vec3::new(torus_x, 20.0, torus_z + 300.0);
```

**Result:**
- Camera circles the torus
- Terrain is fixed in torus space
- Noise will REPEAT every torus_extent meters (this is OK for a toy!)

**Pro:** Simple, guaranteed to work
**Con:** Terrain repeats (same pattern every 512m)

### Approach 2: Infinite Plane with Torus Rendering (Complex)

**Maintain infinite plane coherence but render in torus space.**

**Key insight:** Use camera position as offset for noise sampling

```wgsl
// Vertex position in torus space
let torus_x = f32(x) * spacing;
let torus_z = f32(z) * spacing;

// Calculate which "tile" of the infinite plane we're in
let tile_offset_x = floor(camera_pos.x / torus_extent_x) * torus_extent_x;
let tile_offset_z = floor(camera_pos.z / torus_extent_z) * torus_extent_z;

// Noise sampling in infinite plane space
// Map torus vertex to infinite plane based on camera position
let plane_x = tile_offset_x + torus_x;
let plane_z = tile_offset_z + torus_z;
let height = sample_noise(plane_x, plane_z);

// Write torus-space position
vertices[idx].position = vec3(torus_x, height, torus_z);
```

```rust
// Camera wraps to torus space
let torus_x = camera_pos[0] % torus_extent;
let torus_z = camera_pos[2] % torus_extent;
```

**Result:**
- Camera circles torus (wraps at 512m)
- Noise samples from infinite plane (never repeats!)
- Terrain CHANGES as you circle (different 512m tile each time)

**Wait, this is also wrong!** The terrain would change as you circle, defeating the purpose.

### Approach 3: The ACTUAL Solution - Fixed Noise Field

**The terrain should be FIXED on the torus, not changing!**

Just like a donut in 3ds Max has a fixed bump map, we need fixed noise.

```wgsl
// Vertex in torus space
let torus_x = f32(x) * spacing;
let torus_z = f32(z) * spacing;

// Sample noise at TORUS position (not camera-dependent!)
let height = sample_noise(torus_x, torus_z);

// Write torus position
vertices[idx].position = vec3(torus_x, height, torus_z);
```

```rust
// Camera in torus space
let torus_x = camera_pos[0].rem_euclid(torus_extent);
let torus_z = camera_pos[2].rem_euclid(torus_extent);
let eye = Vec3::new(torus_x, 80.0, torus_z);

// Look ahead in torus space (with wrapping)
let look_ahead = 300.0;
let target_z = (torus_z + look_ahead).rem_euclid(torus_extent);
let target = Vec3::new(torus_x, 20.0, target_z);
```

**This is the 3ds Max model!**

---

## Implementation Plan

### Step 1: Simplify Compute Shader

**Remove all wrapping logic:**

```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let grid_size = params.grid_size;

    if (idx >= grid_size * grid_size) { return; }

    let x = idx % grid_size;
    let z = idx / grid_size;

    // Simple torus-space positions
    let pos_x = f32(x) * params.grid_spacing;
    let pos_z = f32(z) * params.grid_spacing;

    // Sample noise (no camera dependence!)
    let height = sample_noise_at(pos_x, pos_z);

    // Write fixed torus position
    vertices[idx].position = vec3(pos_x, height, pos_z);
    vertices[idx].uv = vec2(f32(x) / f32(grid_size), f32(z) / f32(grid_size));
}
```

**Key: Noise only needs to be computed ONCE (or when audio changes), not every frame!**

### Step 2: Fix Camera to Orbit Torus

```rust
pub fn create_perspective_view_proj_matrix(camera_pos: [f32; 3], torus_extent: f32, aspect: f32) -> [[f32; 4]; 4] {
    use glam::{Mat4, Vec3};

    // Wrap camera to torus space
    let torus_x = camera_pos[0].rem_euclid(torus_extent);
    let torus_z = camera_pos[2].rem_euclid(torus_extent);

    // Camera position in torus space
    let eye = Vec3::new(torus_x, 80.0, torus_z);

    // Look ahead in torus space
    let look_ahead = 300.0;
    let target_z = (torus_z + look_ahead).rem_euclid(torus_extent);
    let target = Vec3::new(torus_x, 20.0, target_z);

    let up = Vec3::Y;
    let view = Mat4::look_at_rh(eye, target, up);
    let proj = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 1.0, 2000.0);

    (proj * view).to_cols_array_2d()
}
```

### Step 3: Only Update Terrain When Audio Changes

**Current:** We recompute terrain every frame (wasteful!)

**Should:** Only dispatch compute shader when audio params change

```rust
// In main.rs render()
let should_update_terrain = audio_changed || first_frame;

if should_update_terrain {
    // Dispatch compute shader
}
```

---

## Summary

**The Fix:**
1. Remove ALL wrapping logic from compute shader
2. Write vertices at fixed torus positions (0 to extent)
3. Sample noise at fixed torus positions (NOT camera-dependent)
4. Wrap camera position to torus space using modulo
5. Wrap camera target to torus space for proper look-ahead

**Result:**
- Camera orbits a fixed torus terrain
- Smooth continuous motion
- No oscillation
- Terrain only updates when audio changes
- Just like 3ds Max!

**Trade-off:**
- Terrain pattern repeats every 512m
- This is acceptable for a toy/prototype
- For production, increase grid size or spacing
## Post-Implementation Discovery: The Real Problem

**TL;DR: We were solving the wrong problem.**

After implementing all approaches and encountering persistent artifacts (split screen, oscillation, seams), we discovered:

**Toroidal topology != flat grid with wrapped edges**

### What We Built
- Flat XZ grid with wrap-around logic
- Result: Visual discontinuities at all seams

### What's Actually Needed
**True 3D torus geometry** using parametric equations - see LEARNINGS.md for details and torus equations.

The geometry must actually BE a torus, not a flat grid pretending to be one.
