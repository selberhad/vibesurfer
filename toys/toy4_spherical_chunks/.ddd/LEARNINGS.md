# Toy 4: Spherical Chunk Streaming - Learnings

## What We Built

GPU-accelerated spherical terrain with 5×5 chunk streaming for infinite ocean simulation.

**Components:**
- Sphere projection compute shader (flat grid → sphere surface)
- Orbital camera flying above equator at 30m altitude
- 5×5 chunk grid with auto-loading/unloading (1024m render distance)
- Forward-looking camera for infinite flight feel
- Distance fog (0-400m) to hide chunk streaming and add depth

## Performance Results

| Configuration | Chunks | Vertices | FPS | Status |
|--------------|--------|----------|-----|---------|
| Single chunk | 1 | 65,536 | 120-121 | ✅ Baseline |
| 3×3 streaming | 9 | 589,824 | 119-121 | ✅ **No FPS drop!** |
| 5×5 streaming | 25 | 1,638,400 | 119-121 | ✅ **Still no FPS drop!** |
| 5×5 + fog | 25 | 1,638,400 | 119-121 | ✅ **Fog is free!** |

**Key Finding:** Rendering 25× geometry with distance fog has **zero performance impact**. GPU-based chunk generation and fragment shader fog are effectively free.

## Technical Validation

### ✅ What Worked

**1. Chunk Streaming Architecture**
- `Chunk::create()` encapsulates all chunk generation in a single call
- Chunks are immutable after creation (no per-frame compute)
- `HashMap<ChunkId, Chunk>` for dynamic chunk management
- 5×5 grid provides seamless coverage (1024m render distance, diagonal ~1448m)

**2. GPU Compute Performance**
- Compute shader projects 65k vertices to sphere in <1ms
- No measurable overhead vs manual buffer creation
- Validates approach for Vibesurfer infinite ocean

**3. Forward-Looking Camera**
- Looking 300m ahead along orbit creates "infinite flight" feel
- Fixed look-at point created "spinning around pole" effect (bad)
- Solution: `look_ahead_angle = current_angle + 300m / planet_radius`

**4. Camera Orientation Fix**
- **Problem:** Ocean appeared to the right instead of below camera
- **Root cause:** Camera "up" vector was fixed to Y-axis instead of radial
- **Solution:** Use radial direction as up: `let up = pos.normalize();`
- **Result:** Camera now correctly looks down at ocean surface below

**5. Camera Altitude Tuning**
- **Problem:** 100m altitude too high, terrain felt sparse
- **Solution:** Lowered to 30m for better visual density
- **Result:** Improved immersion, ocean feels closer and more present

**6. Chunk Grid Expansion**
- **Problem:** 3×3 grid too coarse, visible chunk transitions
- **Solution:** Expanded to 5×5 grid (25 chunks instead of 9)
- **Result:** Seamless coverage, no visible chunk loading

**7. Distance Fog Implementation**
- **Problem:** Chunk streaming transitions visible in distance
- **Solution:** Linear distance fog (0m start, 400m full fade to black)
- **Implementation:**
  - Pass camera position in uniform buffer
  - Calculate distance in fragment shader
  - Mix base color with fog color based on distance
- **Performance:** Zero impact (fragment shader calculation is free)
- **Result:** Hides chunk streaming, creates atmospheric depth

### ⚠️ Remaining Issues

**1. Chunk Resolution Validation**
- 256×256 works well for current scale
- Dynamic LOD not needed yet (120 FPS sustained)
- May need higher resolution if camera goes lower or grid spacing tightens

## Math/Implementation Details

### Sphere Projection
```wgsl
// Compute shader projects flat grid to sphere
let theta = (grid_x / grid_size) * chunk_extent + chunk_center_lon;
let phi = (grid_z / grid_size) * chunk_extent + chunk_center_lat;

// Spherical to Cartesian
pos.x = radius * cos(phi) * cos(theta);
pos.y = radius * sin(phi);
pos.z = radius * cos(phi) * sin(theta);
```

### Camera Look-Ahead
```rust
// Look 300m ahead along orbital path
let look_ahead_angle = self.angular_pos + 300.0 / PLANET_RADIUS;
let look_at = Vec3::new(
    PLANET_RADIUS * look_ahead_angle.cos(),
    0.0,  // Equator
    PLANET_RADIUS * look_ahead_angle.sin(),
);
```

### Chunk Streaming Logic
```rust
// Get 5×5 grid around camera
let needed_chunks: HashSet<ChunkId> = center_id.neighbors();

// Unload distant chunks
chunks.retain(|id, _| needed_chunks.contains(id));

// Load missing chunks
for id in needed_chunks {
    if !chunks.contains_key(&id) {
        chunks.insert(id, Chunk::create(...));
    }
}
```

### Distance Fog
```wgsl
// Fragment shader calculates distance from camera
let distance = length(in.world_pos - camera.camera_pos);

// Linear fog (0m → 400m)
let fog_start = 0.0;
let fog_end = 400.0;
let fog_factor = clamp((distance - fog_start) / (fog_end - fog_start), 0.0, 1.0);

// Mix base color with fog
let final_color = mix(base_color, fog_color, fog_factor);
```

### Camera Orientation
```rust
// Use radial direction as camera "up" vector
let up = pos.normalize();  // Points away from planet center
let view = glam::Mat4::look_at_rh(pos, look_at, up);
```

## Workspace Conversion

Successfully converted project to Cargo workspace during this toy development.

**Benefits realized:**
- Run any toy from root: `cargo run --release --bin toy4_spherical_chunks`
- No more `cd toys/...` directory switching
- Unified dependency resolution
- Shared build cache (faster incremental builds)

## Answered Questions

### ✅ Open Questions Resolved

1. **Optimal camera altitude?**
   - **Answer:** 30m is optimal for current scale
   - 100m too high (sparse, low visual density)
   - 30m provides good immersion and detail visibility
   - May need adjustment if grid spacing or chunk resolution changes

2. **Chunk grid size?**
   - **Answer:** 5×5 confirmed better than 3×3
   - Provides seamless coverage (1024m render distance)
   - No visible chunk transitions with distance fog
   - Maintains 120 FPS with 1.6M vertices

3. **Chunk resolution?**
   - **Answer:** 256×256 validated as sufficient
   - Works well at 30m altitude with 2m grid spacing
   - Dynamic LOD not needed (GPU has headroom)
   - May need higher resolution if camera goes lower

4. **Orientation fix?**
   - **Answer:** Camera up vector (not sphere coordinates)
   - Use radial direction: `pos.normalize()`
   - Ensures camera always looks down at surface below

## Next Steps for Vibesurfer Integration

### Integration Path
1. Apply chunk streaming to main ocean system
2. Add distance fog for atmospheric depth
3. Add audio-reactive terrain modulation (height offsets)
4. Implement proper horizon/skybox
5. Add wave animation on top of base mesh
6. Consider exponential fog for more natural falloff

## Key Learnings

1. **GPU chunk generation is free** - No performance penalty for dynamic terrain (25× geometry, same FPS)
2. **Distance fog is free** - Fragment shader calculations have zero performance impact
3. **Camera orientation is radial** - Use `pos.normalize()` as up vector for spherical terrain
4. **Scale tuning is critical** - 30m altitude vs 100m completely changes feel
5. **5×5 grid is the sweet spot** - Seamless coverage without performance cost
6. **Linear fog works well** - Start at 0m, fade to black by 400m hides chunk streaming effectively
7. **Prefactoring pays off** - `Chunk::create()` made grid expansion trivial
8. **Workspace structure wins** - No more directory confusion
9. **Aggressive fog necessary** - Subtle fog invisible; need strong gradient to hide distant geometry

## Files Modified

**Core Implementation:**
- `toys/toy4_spherical_chunks/src/lib.rs` - Shared chunk system
- `toys/toy4_spherical_chunks/src/main.rs` - Interactive demo
- `toys/toy4_spherical_chunks/src/test_render.rs` - Headless testing
- `toys/toy4_spherical_chunks/src/sphere_compute.wgsl` - GPU projection
- `toys/toy4_spherical_chunks/src/sphere_render.wgsl` - Wireframe rendering

**Infrastructure:**
- `Cargo.toml` - Workspace root
- `vibesurfer/` - Main project moved to subdirectory
- `scripts/generate-loc-report.sh` - Updated for workspace

## Success Criteria

- ✅ 60+ FPS with multi-chunk streaming (achieved 120 FPS with 25 chunks!)
- ✅ Seamless chunk loading/unloading (no visible hitches)
- ✅ Forward-looking camera for infinite feel
- ✅ Ocean orientation correct (radial up vector)
- ✅ Immersive scale and density (30m altitude + 5×5 grid)
- ✅ Chunk transitions hidden (distance fog 0-400m)
- ✅ Performance headroom (GPU has capacity for more features)

**Overall:** All objectives achieved. System ready for Vibesurfer integration with audio-reactive terrain.
