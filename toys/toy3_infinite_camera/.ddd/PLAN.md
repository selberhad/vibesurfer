# Toy Model 3: Toroidal Camera Navigation - Implementation Plan

## Overview

**Goal:** Implement seamless 360° camera navigation on a toroidal terrain mesh with coherent noise sampling.

**Scope:** Implement toroidal wrapping in compute shader and add keyboard controls to test various motion patterns.

**Starting point:** Toy3 is a copy of toy2 with refactored infrastructure:
- ✅ `CameraState` struct handles position, velocity, and delta time updates
- ✅ `TerrainParams::new()` helper with torus_extent fields already added
- ✅ Camera matrix is camera-relative (camera at origin in view space)
- ✅ Velocity-based position update already implemented

**What's left:**
1. Implement toroidal wrapping math in compute shader
2. Test and validate with keyboard controls for different motion patterns

**Priorities:**
1. Correct toroidal wrapping (nearest distance on torus)
2. World-space noise coherence (same world XZ = same height)
3. Visual validation at each step
4. Performance maintained (≥60 FPS)

**Methodology:**
- TDD via visual validation: run binary after each step, verify terrain looks correct
- Manual testing: keyboard controls for motion patterns (forward, diagonal, stop, circle)
- Commit after each step with conventional format

---

## Step 1: Implement Toroidal Wrapping in Compute Shader

### Goal
Replace broken per-vertex wrapping (lines 132-142 in terrain_compute.wgsl) with correct toroidal coordinate math.

### Step 1.a: Write Tests
- **Visual test:** Run binary, terrain should render centered on camera
- **Validation:**
  - No visible seams or discontinuities
  - Terrain appears continuous (not stretched)
  - No phantom lines (toy2 regression)
  - FPS ≥60

### Step 1.b: Implement

**Tasks:**

1. Replace lines 132-142 in `src/terrain_compute.wgsl` with toroidal wrapping:

```wgsl
// Calculate grid-local position (0 to torus_extent)
let local_x = f32(x) * params.grid_spacing;
let local_z = f32(z) * params.grid_spacing;

// Wrap camera position to torus space (modulo operation)
let camera_torus_x = params.camera_pos.x - floor(params.camera_pos.x / params.torus_extent_x) * params.torus_extent_x;
let camera_torus_z = params.camera_pos.z - floor(params.camera_pos.z / params.torus_extent_z) * params.torus_extent_z;

// Calculate offset from camera (raw difference)
var dx = local_x - camera_torus_x;
var dz = local_z - camera_torus_z;

// Wrap to nearest distance on torus (handle seam crossing)
let half_extent_x = params.torus_extent_x * 0.5;
let half_extent_z = params.torus_extent_z * 0.5;

if (dx > half_extent_x) { dx -= params.torus_extent_x; }
if (dx < -half_extent_x) { dx += params.torus_extent_x; }
if (dz > half_extent_z) { dz -= params.torus_extent_z; }
if (dz < -half_extent_z) { dz += params.torus_extent_z; }

// For noise sampling, use unwrapped world coordinates
let world_x = params.camera_pos.x + dx;
let world_z = params.camera_pos.z + dz;
```

2. Update noise sampling to use unwrapped world coordinates (lines 145-152):

```wgsl
// Sample base terrain at world coordinates (coherent across torus wrapping)
let base_coord_x = world_x * 0.1 * params.base_frequency;
let base_coord_z = world_z * 0.1 * params.base_frequency;
let base_height = simplex3d(vec3<f32>(base_coord_x, base_coord_z, 0.0)) * params.base_amplitude;

// Sample detail layer at world coordinates
let detail_coord_x = world_x * 0.1 * params.detail_frequency;
let detail_coord_z = world_z * 0.1 * params.detail_frequency;
let detail_height = simplex3d(vec3<f32>(detail_coord_x, detail_coord_z, params.time)) * params.detail_amplitude;
```

3. Update vertex output to use camera-relative position (line 158):

```wgsl
// Write camera-relative position (dx, dz) not world position
vertices[idx].position = vec3<f32>(dx, height, dz);
```

4. Update window title in `src/main.rs:534`:

```rust
.with_title("Toy 3: Toroidal Camera Navigation")
```

### Success Criteria
- [ ] Code compiles without shader errors
- [ ] Terrain renders centered on camera
- [ ] No phantom lines (regression from toy2)
- [ ] Terrain appears smooth and continuous
- [ ] FPS ≥60

**Commit:** `feat(toy3): Step 1 - implement toroidal coordinate wrapping`

---

## Step 2: Test Continuous Forward Motion and Wrapping

### Goal
Verify terrain scrolls smoothly as camera moves forward, with no seams at torus boundaries.

### Step 2.a: Write Tests
- **Visual test:** Run for 120 seconds (camera travels 1200m, wraps torus once at 1024m)
- **Validation:**
  - Terrain scrolls smoothly backward
  - No seam visible when crossing z=1024m boundary
  - No visual pops or discontinuities
  - FPS ≥60 throughout

### Step 2.b: Implement

**Tasks:**

1. Add console logging to track torus wrapping in `src/main.rs` after line 432:

```rust
// Update camera position based on velocity
self.camera.update();

// Log torus wrapping (every 1024m)
if self.camera.position[2] > 0.0 && (self.camera.position[2] as u32) % 1024 < 10 {
    println!("Camera at z = {:.1}m (torus wrap at 1024m intervals)", self.camera.position[2]);
}
```

2. Run binary and observe for 120 seconds
3. Watch console for wrap events
4. Visually confirm no seams at boundaries

### Success Criteria
- [ ] Terrain scrolls smoothly
- [ ] Console logs torus position updates
- [ ] No visible seam at wrap boundaries (z=1024m, z=2048m)
- [ ] FPS avg ≥60, min >50
- [ ] No visual artifacts or stuttering

**Commit:** `feat(toy3): Step 2 - validate forward motion with torus wrapping`

---

## Step 3: Add Keyboard Controls for Motion Testing

### Goal
Add keyboard controls to test diagonal motion, stopping, and circular motion patterns.

### Step 3.a: Write Tests
- **Manual tests:**
  1. Press '1': Forward motion (default)
  2. Press '2': Diagonal motion (northeast)
  3. Press 'Space': Stop/resume
  4. Press '3': Circular motion (orbit center)
- **Validation:**
  - Each mode produces expected visual motion
  - Transitions are smooth (no pops)
  - FPS ≥60 in all modes

### Step 3.b: Implement

**Tasks:**

1. Add motion mode enum to `src/main.rs` around line 85:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum MotionMode {
    Forward,
    Diagonal,
    Stopped,
    Circular,
}

struct App {
    // ... existing fields ...
    camera: CameraState,
    motion_mode: MotionMode,
    circle_start_time: f32,
    window: Arc<Window>,
}
```

2. Initialize in `App::new()` around line 408:

```rust
camera: CameraState::new([0.0, 0.0, 0.0], [0.0, 0.0, 10.0]),
motion_mode: MotionMode::Forward,
circle_start_time: 0.0,
window,
```

3. Add keyboard handler in `window_event()` around line 547:

```rust
WindowEvent::KeyboardInput {
    event:
        KeyEvent {
            state: ElementState::Pressed,
            physical_key: PhysicalKey::Code(code),
            ..
        },
    ..
} => {
    if let Some(app) = &mut self.app {
        match code {
            KeyCode::Digit1 => {
                app.motion_mode = MotionMode::Forward;
                app.camera.set_velocity([0.0, 0.0, 10.0]);
                println!("Mode: Forward (10 m/s)");
            }
            KeyCode::Digit2 => {
                app.motion_mode = MotionMode::Diagonal;
                app.camera.set_velocity([7.07, 0.0, 7.07]);
                println!("Mode: Diagonal northeast (~10 m/s)");
            }
            KeyCode::Space => {
                if app.motion_mode == MotionMode::Stopped {
                    app.motion_mode = MotionMode::Forward;
                    app.camera.set_velocity([0.0, 0.0, 10.0]);
                    println!("Mode: Forward (resumed)");
                } else {
                    app.motion_mode = MotionMode::Stopped;
                    app.camera.set_velocity([0.0, 0.0, 0.0]);
                    println!("Mode: Stopped");
                }
            }
            KeyCode::Digit3 => {
                app.motion_mode = MotionMode::Circular;
                app.circle_start_time = app.start_time.elapsed().as_secs_f32();
                println!("Mode: Circular (200m radius, 60s period)");
            }
            _ => {}
        }
    }
}
```

4. Add circular motion update in `render()` before camera update:

```rust
// Handle circular motion mode
if self.motion_mode == MotionMode::Circular {
    let circle_time = time - self.circle_start_time;
    let radius = 200.0;
    let period = 60.0;
    let theta = (circle_time / period) * 2.0 * std::f32::consts::PI;

    // Update position directly for smooth circle
    self.camera.position[0] = radius * theta.cos();
    self.camera.position[2] = radius * theta.sin();

    // Set velocity tangent to circle
    let velocity_mag = 2.0 * std::f32::consts::PI * radius / period;
    self.camera.set_velocity([
        -velocity_mag * theta.sin(),
        0.0,
        velocity_mag * theta.cos(),
    ]);
} else {
    // Normal velocity-based update
    self.camera.update();
}
```

5. Test each mode:
   - Run binary
   - Press '1', '2', 'Space', '3' and verify motion
   - Observe console output and FPS

### Success Criteria
- [ ] Keyboard '1': Forward motion works
- [ ] Keyboard '2': Diagonal motion works (terrain scrolls southwest)
- [ ] Keyboard 'Space': Stop/resume works (no drift when stopped)
- [ ] Keyboard '3': Circular motion works (completes orbit)
- [ ] Mode transitions smooth (no visual pops)
- [ ] FPS ≥60 in all modes
- [ ] Console prints mode changes

**Commit:** `feat(toy3): Step 3 - add keyboard controls for motion testing`

---

## Step 4: Performance Validation and Noise Coherence Test

### Goal
Confirm performance meets targets across all motion patterns and validate terrain coherence.

### Step 4.a: Write Tests
- **Benchmark:** Run each motion mode for 60 seconds, observe min/avg/max FPS
- **Modes:** Forward, diagonal, circular, stopped
- **Coherence test:** Complete full circular orbit, verify terrain looks same at start/end
- **Validation:** All modes achieve FPS ≥60

### Step 4.b: Implement

**Tasks:**

1. Add FPS summary command in keyboard handler:

```rust
KeyCode::KeyP => {
    let (min, avg, max) = app.fps_tracker.stats();
    println!("=== FPS Summary ===");
    println!("Min: {:.1}, Avg: {:.1}, Max: {:.1}", min, avg, max);
}
```

2. Test procedure:
   - Run binary in release mode: `cargo run --release --bin toy3`
   - Test each mode for 60 seconds:
     - Press '1', wait 60s, press 'P' (record FPS)
     - Press '2', wait 60s, press 'P' (record FPS)
     - Press '3', wait 60s, press 'P' (record FPS - full orbit)
     - Press 'Space', wait 10s, press 'P' (record FPS while stopped)
   - For circular mode: visually compare terrain at t=0s and t=60s (should look identical)

3. Document results in LEARNINGS.md

### Success Criteria
- [ ] Forward motion: FPS avg ≥60, min >50
- [ ] Diagonal motion: FPS avg ≥60, min >50
- [ ] Circular motion: FPS avg ≥60, min >50
- [ ] Stopped: FPS avg ≥60 (GPU still computing)
- [ ] Circular orbit coherence: terrain looks same after 60s
- [ ] No X or Z wrapping artifacts (both axes wrap correctly)

**Commit:** `feat(toy3): Step 4 - performance validation and coherence test`

---

## Final Validation

### All Tests Pass
- [ ] Forward motion smooth with no seams (120s test)
- [ ] Diagonal motion wraps correctly in both X and Z axes
- [ ] Stop/resume works with no drift
- [ ] Circular motion completes full orbit with terrain coherence
- [ ] Performance ≥60 FPS in all modes

### Ready for Integration
- [ ] LEARNINGS.md written with findings and performance data
- [ ] README.md updated with keyboard controls
- [ ] Code ready to reference for main vibesurfer integration

---

## Implementation Notes

**Key files to modify:**
- `src/terrain_compute.wgsl` - Toroidal wrapping logic (lines 132-158)
- `src/main.rs` - Keyboard controls, motion modes, circular motion update

**What was already refactored:**
- `CameraState` struct (lib.rs:22-53) - Handles velocity, position, delta time
- `TerrainParams::new()` (lib.rs:83-98) - Simplified param construction with torus extents
- `create_perspective_view_proj_matrix()` (lib.rs:87-108) - Camera-relative (camera at origin)

**Integration points for main codebase:**
- Toroidal wrapping shader code (copy compute shader logic)
- `CameraState` pattern (reference for player controls)
- Performance validated at 512×512 grid (60+ FPS confirmed)

**Out of scope:**
- GPU vertex readback (manual visual validation sufficient)
- Automated test framework (keyboard testing works for toy)
- Recording/capture (not needed for validation)
- Player physics integration (deferred to main)
