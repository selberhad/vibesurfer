# Camera System Refactor Plan

**Goal**: Eliminate "grid flowing" simulation and make all cameras move through actual world space, compatible with GPU terrain generation.

**Status**: Ready to implement
**Related**: REFACTOR_PLAN.md (GPU terrain Phase 1 complete, Phase 2 blocked on this)

**Note**: CPU terrain generation is being removed. This refactor makes all cameras move through world space, compatible with GPU-only terrain generation.

---

## Problem Statement

**Current GPU terrain bug**: After ~20 seconds, terrain degenerates into flat plane.

**Root cause**: `effective_camera_pos` calculation in main.rs:193-198 creates unbounded accumulation:
```rust
let effective_camera_pos = if let Some(sim_vel) = self.camera.get_simulated_velocity(time_s) {
    camera_pos + sim_vel * time_s  // After 20s: [0, 101, 3000]
} else {
    camera_pos
};
```

GPU shader samples noise at `world_x = grid_x - camera_pos.x`, causing:
- Float precision loss at large coordinates
- Noise sampling outside intended range
- Terrain flattening/degeneration

---

## Current Architecture (Hybrid Approach)

### Camera Modes

| Mode | Camera Motion | Grid Motion | Implementation |
|------|--------------|-------------|----------------|
| **Basic** | Moves through world (`z = time * speed`) | Stationary | `compute_basic_path()` |
| **Cinematic** | Moves through world (procedural) | Stationary | `compute_cinematic_path()` |
| **Fixed** | Stays at origin | Flows backward (`simulated_velocity`) | `compute_fixed_path()` + `get_simulated_velocity()` |
| **Floating** | Stays at origin | Flows backward | `compute_floating_path()` + `get_simulated_velocity()` |

**CPU terrain generation**: Handles both approaches
- For Basic/Cinematic: Grid vertices are static, camera moves
- For Fixed/Floating: Vertices flow backward via `ocean.update()` in mesh.rs:134-148

**GPU terrain generation**: Only handles static grid
- Shader calculates `world_x = grid_x - camera_pos.x`
- Expects `camera_pos` to be actual world position, not accumulated offset

---

## Target Architecture (Unified Approach)

**Principle**: All cameras move through infinite world space. Grid represents a local window around camera.

### Camera Modes (Refactored)

| Mode | Camera Motion | Grid Behavior | World Coords |
|------|--------------|---------------|--------------|
| **Basic** | Straight line at constant altitude | Local window follows camera | `z = time * speed` |
| **Cinematic** | Procedural path (sweeps, climbs) | Local window follows camera | Complex time-based motion |
| **Fixed** | Moves forward at constant velocity | Local window follows camera | `z = time * simulated_velocity` |
| **Floating** | Follows terrain at fixed height | Local window follows camera | `z = time * velocity(t)` |

**Key insight**: "Fixed" camera is now just "Basic with zero lateral motion" - the camera still moves through world space, it just has a simple straight path.

---

## Implementation Changes

### 1. Camera System (camera.rs)

**Remove**:
- `get_simulated_velocity()` method (lines 96-112)
- Concept of "grid flowing" from all camera types

**Modify**:
- `FixedCamera::compute_fixed_path()` → Returns time-based position
  ```rust
  fn compute_fixed_path(p: &FixedCamera, time_s: f32) -> (Vec3, Vec3) {
      // Camera moves forward through world space
      let eye = Vec3::new(
          p.position[0],  // X: stays constant
          p.position[1],  // Y: stays at elevation
          time_s * p.simulated_velocity  // Z: moves forward
      );

      // Target moves with camera
      let target = Vec3::new(
          p.target[0],
          p.target[1],
          eye.z + (p.target[2] - p.position[2])  // Maintain relative offset
      );

      (eye, target)
  }
  ```

- `FloatingCamera::compute_floating_path()` → Camera actually moves
  ```rust
  fn compute_floating_path<F>(p: &FloatingCamera, time_s: f32, get_height: F) -> (Vec3, Vec3)
  where
      F: Fn(f32, f32) -> f32,
  {
      // Calculate distance traveled with acceleration
      let distance = p.initial_velocity * time_s + 0.5 * p.acceleration * time_s * time_s;

      // Camera position in world space
      let x = p.position_xz[0];
      let z = p.position_xz[1] + distance;  // Actually moves forward

      // Query terrain at camera's actual position
      let terrain_height = get_height(x, z);
      let y = terrain_height + p.height_above_terrain_m;

      let eye = Vec3::new(x, y, z);

      // Look-at target
      let target_x = x;
      let target_z = z + p.look_ahead_m;
      let target_terrain_height = get_height(target_x, target_z);
      let target_y = target_terrain_height + p.height_above_terrain_m * 0.6;

      let target = Vec3::new(target_x, target_y, target_z);

      (eye, target)
  }
  ```

**Add**:
- Method signature change: All path computation takes `time_s` parameter
  ```rust
  fn compute_fixed_path(p: &FixedCamera, time_s: f32) -> (Vec3, Vec3)
  ```

### 2. Main Render Loop (main.rs)

**Remove**:
- `effective_camera_pos` calculation (lines 192-198)
- All references to `get_simulated_velocity()`

**Modify**:
- GPU terrain path (lines 202-233):
  ```rust
  #[cfg(feature = "gpu-terrain")]
  let (amplitude, frequency, line_width, index_count) = {
      // Compute audio-modulated parameters
      let amplitude = self.ocean.physics.detail_amplitude_m
          + audio_bands.low * self.ocean.mapping.bass_to_amplitude_scale;
      let frequency = self.ocean.physics.detail_frequency
          + audio_bands.mid * self.ocean.mapping.mid_to_frequency_scale;
      let line_width = self.ocean.physics.base_line_width
          + audio_bands.high * self.ocean.mapping.high_to_glow_scale;

      // Create terrain params for GPU (use actual camera position)
      let terrain_params = vibesurfer::params::TerrainParams {
          base_amplitude: self.ocean.physics.base_terrain_amplitude_m,
          base_frequency: self.ocean.physics.base_terrain_frequency,
          detail_amplitude: amplitude,
          detail_frequency: frequency,
          camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z],  // ← Direct camera pos
          _padding1: 0.0,
          grid_size: self.ocean.physics.grid_size as u32,
          grid_spacing: self.ocean.physics.grid_spacing_m,
          time: time_s * self.ocean.physics.wave_speed,
          _padding2: 0.0,
      };

      // Dispatch GPU compute shader
      render_system.dispatch_terrain_compute(&terrain_params, self.ocean.physics.grid_size as u32);

      // Use all indices (no phantom line filtering in Phase 1)
      let index_count = self.ocean.grid.indices.len() as u32;

      (amplitude, frequency, line_width, index_count)
  };
  ```

- CPU terrain path: Update to use camera_pos directly
  ```rust
  #[cfg(not(feature = "gpu-terrain"))]
  let (amplitude, frequency, line_width, index_count) = {
      // CPU path: pass actual camera position
      let (amplitude, frequency, line_width) =
          self.ocean.update(time_s, &audio_bands, camera_pos);  // ← Not effective_camera_pos

      // Update ocean vertices and indices
      render_system.update_vertices(&self.ocean.grid.vertices);
      render_system.update_indices(&self.ocean.grid.filtered_indices);

      let index_count = self.ocean.grid.filtered_indices.len() as u32;

      (amplitude, frequency, line_width, index_count)
  };
  ```

### 3. Ocean System (ocean/system.rs & ocean/mesh.rs)

**Modify**:
- `OceanSystem::update()` signature remains same, but semantics change:
  - `camera_pos` is now actual world position (not accumulated offset)
  - Vertices flow relative to camera's current world position

- `OceanGrid::update()` in mesh.rs:123-196:
  - Current: Flows all vertices backward by camera delta
  - New: Recenters grid around camera position each frame

  **Current logic** (lines 134-148):
  ```rust
  // Flow grid forward (camera "moves" backward relative to grid)
  let camera_delta_z = camera_pos.z - self.last_camera_pos.z;

  for vertex in &mut self.vertices {
      vertex.position[2] += camera_delta_z;
  }

  // Wrap vertices that went too far
  if wrapped {
      vertex.position[2] -= grid_extent;
  }
  ```

  **New logic**:
  ```rust
  // Grid is always centered around camera
  // Vertices represent offsets from camera position
  // No flowing, no wrapping - GPU handles this in shader

  // Calculate world position for noise sampling:
  let world_x = camera_pos.x + vertex.position[0];
  let world_z = camera_pos.z + vertex.position[2];

  // Sample noise at world coordinates
  let base_height = sample_base_terrain(world_x, world_z);
  let detail_height = sample_detail_terrain(world_x, world_z, time_s);

  vertex.position[1] = base_height + detail_height;
  ```

**Note**: CPU version may still need vertex flowing for performance (avoid recomputing base terrain). GPU version doesn't need this since compute is so fast.

### 4. GPU Compute Shader (terrain_compute.wgsl)

**Current logic** (lines 133-144):
```wgsl
// Calculate world-space position (grid pos - camera offset)
let grid_extent = f32(grid_size) * params.grid_spacing;
let half_size = grid_extent * 0.5;
let grid_x = f32(x) * params.grid_spacing - half_size;
let grid_z = f32(z) * params.grid_spacing - half_size;

var world_x = grid_x - params.camera_pos.x;  // ← Problem: assumes camera at origin
var world_z = grid_z - params.camera_pos.z;

// Toroidal wrapping
if (world_z < wrap_threshold) {
    world_z += grid_extent;
}
```

**New logic**:
```wgsl
// Grid is local window around camera
// Grid coordinates are offsets from camera position
let grid_extent = f32(grid_size) * params.grid_spacing;
let half_size = grid_extent * 0.5;
let grid_x = f32(x) * params.grid_spacing - half_size;
let grid_z = f32(z) * params.grid_spacing - half_size;

// World position is camera + grid offset
let world_x = params.camera_pos.x + grid_x;
let world_z = params.camera_pos.z + grid_z;

// No wrapping needed - grid moves with camera
// Infinite terrain from noise function's natural tiling
```

**Alternative with explicit wrapping** (if we want bounded world coords):
```wgsl
// Wrap world coordinates to prevent float precision loss
let WORLD_SIZE = 10000.0;  // Large enough to avoid visible seams
let world_x = (params.camera_pos.x + grid_x) % WORLD_SIZE;
let world_z = (params.camera_pos.z + grid_z) % WORLD_SIZE;
```

---

## Migration Strategy

### Phase A: Update Camera System (1 session)

**Tasks**:
1. Modify `FixedCamera::compute_fixed_path()` to take `time_s` parameter
2. Update `FloatingCamera::compute_floating_path()` to move camera through world
3. Remove `get_simulated_velocity()` method
4. Update `CameraSystem::compute_position_and_target()` to pass `time_s` to Fixed path

**Validation**:
- Build succeeds
- All camera presets render (visuals may be broken, that's expected)
- No compilation errors

### Phase B: Update Main Loop (1 session)

**Tasks**:
1. Remove `effective_camera_pos` calculation
2. Update GPU path to use `camera_pos` directly
3. Remove CPU terrain generation fallback (delete `#[cfg(not(feature = "gpu-terrain"))]` branch)

**Validation**:
- GPU mode: Terrain should stay stable beyond 20 seconds
- No compilation errors

### Phase C: Update GPU Shader (1 session)

**Tasks**:
1. Change world coordinate calculation: `world_x = camera_pos.x + grid_x`
2. Remove toroidal wrapping logic (or add world-wrap if needed)
3. Test with various camera positions to verify no degeneration

**Validation**:
- GPU terrain stable at time > 60 seconds
- Terrain scrolls smoothly as camera moves
- No visible seams or discontinuities

**Note**: Phase D (Update CPU Ocean System) has been removed. We are going GPU-only for terrain generation. CPU will only read back GPU-generated heights for physics queries (implemented in REFACTOR_PLAN.md Phase 2).

---

## Testing Plan

### Visual Tests

**Fixed Camera** (60 second recording):
- [ ] Terrain stable throughout (no degeneration)
- [ ] Hills scroll smoothly
- [ ] Audio reactivity preserved

**Basic Camera** (30 second recording):
- [ ] Camera moves forward smoothly
- [ ] Terrain scrolls correctly
- [ ] No regressions from refactor

**Cinematic Camera** (30 second recording):
- [ ] Complex camera paths work
- [ ] Terrain follows camera correctly

**Floating Camera** (30 second recording):
- [ ] Camera follows terrain contour
- [ ] Accelerates as expected
- [ ] Terrain height queries work

### Performance Tests

- [ ] GPU mode: 100+ FPS at 1024×1024 (should be unchanged)
- [ ] CPU mode: Baseline FPS (12-15 FPS, acceptable degradation vs old CPU mode)

### Correctness Tests

**World coordinate stability**:
- [ ] Render at `time = 0s`, capture frame
- [ ] Render at `time = 1000s`, capture frame
- [ ] Verify terrain pattern matches (accounting for camera movement)

**Precision test**:
- [ ] Run for 300 seconds (5 minutes)
- [ ] Verify no visual artifacts from float precision loss
- [ ] Terrain remains coherent

---

## Known Risks

### 1. ~~CPU Performance Regression~~ (N/A - removing CPU mode)

**Removed**: We are going GPU-only. No CPU terrain generation to optimize.

### 2. World Coordinate Precision Loss

**Risk**: After camera travels very far (e.g., 100,000 meters), float precision degrades.

**Mitigation**:
- Add world-wrap: `world_coord % LARGE_VALUE`
- Choose LARGE_VALUE big enough to avoid visible seams (10km+)
- Document limitation in physics queries

### 3. Phantom Lines Reappear

**Risk**: Removing toroidal wrapping might expose phantom lines in different way.

**Mitigation**:
- Phase C: Test extensively with recordings
- May need different wrapping strategy
- Defer to future "fix phantom lines" task

### 4. Floating Camera Physics Query

**Risk**: Floating camera queries terrain height at `camera.z` position. If camera moves through world, query coordinates change.

**Mitigation**:
- This is actually correct behavior! Query at actual camera position
- Physics readback (Phase 2 of GPU refactor) will handle this

---

## Success Criteria

**Phase A (Camera System)**:
- [ ] All camera path functions accept `time_s`
- [ ] `get_simulated_velocity()` removed
- [ ] Compiles without errors

**Phase B (Main Loop)**:
- [ ] `effective_camera_pos` removed
- [ ] GPU and CPU paths use `camera_pos` directly
- [ ] Compiles without errors

**Phase C (GPU Shader)**:
- [ ] World coordinates calculated as `camera + grid_offset`
- [ ] 60+ second recordings show stable terrain
- [ ] No degeneration artifacts

**Overall**:
- [ ] Fixed camera: 60s recording, stable terrain
- [ ] All presets: 30s recording, correct motion
- [ ] GPU mode: 120 FPS
- [ ] No visual regressions

---

## Files to Modify

### Modified Files
- [ ] `src/camera.rs` - Update path computation methods
- [ ] `src/params/camera.rs` - Update FixedCamera/FloatingCamera structs (optional)
- [ ] `src/main.rs` - Remove effective_camera_pos, update terrain dispatch
- [ ] `src/terrain_compute.wgsl` - Change world coord calculation
- [ ] `src/ocean/system.rs` - Update camera_pos semantics documentation
- [ ] `src/ocean/mesh.rs` - Refactor vertex flowing (CPU mode)

### No Changes Needed
- `src/rendering.rs` - Render pipeline unchanged
- `src/params/ocean.rs` - Terrain params unchanged
- Basic/Cinematic camera logic - Already correct

---

## Next Steps

1. **Review this plan** with human for approval/modifications
2. **Phase A**: Update camera system
3. **Phase B**: Update main loop (remove CPU terrain generation)
4. **Phase C**: Update GPU shader

**Estimated timeline**: 3 sessions (1 per phase)

**Blocking dependency**: None (can start immediately)

**Related work**: After completion, can proceed to GPU terrain Phase 2 (physics readback)

---

## Notes

**Philosophy**: Camera moves through infinite world. Grid is a local window. Noise function provides infinite terrain.

**Simplification**: Removes the mental model split between "moving camera" (Basic/Cinematic) and "flowing grid" (Fixed/Floating).

**GPU-first**: This design optimizes for GPU terrain generation (Phase 1). CPU mode is secondary.

**Future work**:
- Physics readback (GPU Phase 2) will need world-coordinate-aware terrain queries
- Player skiing physics will query terrain at player's world position
- Phantom line elimination (future phase)
