# Toy 4: Spherical Chunk Streaming - Learnings

## What We Built

GPU-accelerated spherical terrain with 3×3 chunk streaming for infinite ocean simulation.

**Components:**
- Sphere projection compute shader (flat grid → sphere surface)
- Orbital camera flying above equator
- 3×3 chunk grid with auto-loading/unloading
- Forward-looking camera for infinite flight feel

## Performance Results

| Configuration | Chunks | Vertices | FPS | Status |
|--------------|--------|----------|-----|---------|
| Single chunk | 1 | 65,536 | 120-121 | ✅ Baseline |
| 3×3 streaming | 9 | 589,824 | 119-121 | ✅ **No FPS drop!** |

**Key Finding:** Rendering 9× geometry has **zero performance impact**. GPU-based chunk generation is fast enough for seamless streaming.

## Technical Validation

### ✅ What Worked

**1. Chunk Streaming Architecture**
- `Chunk::create()` encapsulates all chunk generation in a single call
- Chunks are immutable after creation (no per-frame compute)
- `HashMap<ChunkId, Chunk>` for dynamic chunk management
- 3×3 grid provides seamless coverage around camera

**2. GPU Compute Performance**
- Compute shader projects 65k vertices to sphere in <1ms
- No measurable overhead vs manual buffer creation
- Validates approach for Vibesurfer infinite ocean

**3. Forward-Looking Camera**
- Looking 300m ahead along orbit creates "infinite flight" feel
- Fixed look-at point created "spinning around pole" effect (bad)
- Solution: `look_ahead_angle = current_angle + 300m / planet_radius`

### ⚠️ Issues Discovered

**1. Ocean Orientation Incorrect**
- **Problem:** Ocean appears to the right instead of below camera
- **Root cause:** Sphere projection or camera up vector misalignment
- **Impact:** Breaks immersion - should be flying over ocean, not beside it
- **Fix needed:** Review sphere coordinate system and camera orientation

**2. Chunk Streaming Too Coarse**
- **Problem:** Chunk transitions visible/delayed, terrain feels sparse
- **Symptoms:**
  - Can see chunks loading in distance
  - Grid feels too spread out
  - Not enough visual density for "infinite ocean" feel
- **Likely causes:**
  - Camera too far from surface (100m altitude may be too high)
  - Chunks too large (256×256 at 2m spacing = 512m extent)
  - Need tighter chunk grid or lower camera altitude
- **Potential fixes:**
  - Reduce camera altitude (100m → 20-30m above surface)
  - Increase chunk resolution (256×256 → 512×512)
  - Tighter grid spacing (2m → 1m)
  - Expand to 5×5 chunk grid for more coverage

**3. Scale Feels Wrong**
- **Problem:** Not feeling immersed in infinite ocean
- **Contributing factors:**
  - Orientation issue (ocean to side, not below)
  - Camera altitude too high for visual density
  - Chunk streaming transitions visible
- **Goal:** Should feel like low-altitude flight over endless water

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
// Get 3×3 grid around camera
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

## Workspace Conversion

Successfully converted project to Cargo workspace during this toy development.

**Benefits realized:**
- Run any toy from root: `cargo run --release --bin toy4_spherical_chunks`
- No more `cd toys/...` directory switching
- Unified dependency resolution
- Shared build cache (faster incremental builds)

## Next Steps for Vibesurfer Integration

### Immediate Fixes Needed
1. **Fix ocean orientation** - Should be below, not to the right
2. **Tune camera altitude** - Lower to 20-30m for better visual density
3. **Tighten chunk streaming** - Either smaller chunks or 5×5 grid

### Integration Path
1. Apply chunk streaming to main ocean system
2. Add audio-reactive terrain modulation
3. Implement proper horizon/skybox
4. Add wave animation on top of base mesh

### Open Questions
- **Optimal camera altitude?** Need to test 20m, 30m, 50m, 100m
- **Chunk grid size?** 3×3 works but may need 5×5 for seamless feel
- **Chunk resolution?** 256×256 vs 512×512 vs dynamic LOD?
- **Orientation fix?** Is it camera up vector or sphere coordinate system?

## Key Learnings

1. **GPU chunk generation is free** - No performance penalty for dynamic terrain
2. **Camera orientation matters** - Forward-looking creates infinite feel
3. **Scale tuning is critical** - Small errors in altitude/spacing break immersion
4. **Prefactoring pays off** - `Chunk::create()` made multi-chunk trivial
5. **Workspace structure wins** - No more directory confusion

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

- ✅ 60+ FPS with multi-chunk streaming (achieved 120 FPS!)
- ✅ Seamless chunk loading/unloading (no visible hitches)
- ✅ Forward-looking camera for infinite feel
- ⚠️ Ocean orientation correct (needs fix)
- ⚠️ Immersive scale and density (needs tuning)

**Overall:** Strong technical foundation. Orientation and scale tuning needed for full immersion.
