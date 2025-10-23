# Toy 4: Spherical Chunk Streaming - Learnings

## Core Insights

**GPU compute is effectively free at this scale**
- 1.6M vertices (25 chunks) vs 65k vertices (1 chunk): same 120 FPS
- Chunk generation via compute shader has no measurable overhead
- Fragment shader fog calculations add zero performance cost
- Bottleneck is not GPU; headroom exists for more features

**Exponential fog >> linear fog for hiding streaming**
- Linear fog creates visible "drawing in" at boundaries (even with 5×5 grid)
- Exponential fog (`1.0 - exp2(-density * distance)`) gives smooth natural falloff
- Density 0.015 works well: gradual near, stronger far
- Chunk grid size doesn't matter if fog is wrong; 3×3 sufficient with exponential

**Camera orientation on sphere is radial, not Y-up**
- Fixed Y-up makes ocean appear sideways
- Use `pos.normalize()` as camera up vector (points away from planet center)
- Creates correct "looking down at surface below" orientation

**Forward-looking camera needs angular offset**
- `look_at = camera_angle + offset_meters / planet_radius`
- Looking at fixed point creates "spinning around pole" (wrong)
- Looking ahead 300m creates "infinite flight" feel (correct)

**Scale tuning changes everything**
- 30m altitude vs 100m completely different immersion level
- Lower = denser visual field, better presence
- Must tune altitude + chunk grid spacing + noise frequency together

**Noise frequency must match desired feature spacing**
- Analytical calculation: `freq = 1.0 / (desired_spacing_m / planet_radius)`
- Example: 75m hills → `freq = 1.0 / (75 / 1_000_000) = 13,333`
- Sampling at lat/lon ensures global consistency across chunk boundaries
- Multi-octave works: base layer (large features) + detail layer (small features)

**Immutable chunks simplify streaming**
- `Chunk::create()` does all work upfront (compute dispatch, buffer creation)
- Chunks never update after creation
- Streaming = `HashMap<ChunkId, Chunk>` with retain/insert
- No per-frame compute overhead

## Implementation Details

### Sphere projection (compute shader)
```wgsl
let theta = (grid_x / grid_size) * chunk_extent + chunk_center_lon;
let phi = (grid_z / grid_size) * chunk_extent + chunk_center_lat;
pos.x = radius * cos(phi) * cos(theta);
pos.y = radius * sin(phi);
pos.z = radius * cos(phi) * sin(theta);
```

### Exponential fog (fragment shader)
```wgsl
let distance = length(in.world_pos - camera.camera_pos);
let fog_factor = 1.0 - exp2(-fog_density * distance);
let final_color = mix(base_color, fog_color, fog_factor);
```

### Camera radial up vector
```rust
let up = pos.normalize();  // Radial direction from planet center
let view = glam::Mat4::look_at_rh(pos, look_at, up);
```

### Chunk streaming
```rust
let needed_chunks: HashSet<ChunkId> = center_id.neighbors();
chunks.retain(|id, _| needed_chunks.contains(id));  // Unload distant
for id in needed_chunks {
    chunks.entry(id).or_insert_with(|| Chunk::create(...));  // Load missing
}
```

## Configuration (validated)
- Planet radius: 1,000,000m (1000km)
- Camera altitude: 30m
- Chunk grid: 3×3 (9 chunks, 589k vertices)
- Chunk resolution: 256×256 vertices
- Grid spacing: 2m
- Fog: Exponential, density 0.015
- Noise: Base (10m @ 75m spacing) + Detail (3m @ 20m spacing)

## Open Questions
- Does chunk grid need latitude expansion at higher latitudes? (Currently only longitude streaming)
- What's the minimum chunk resolution before terrain aliasing becomes visible?
- Can noise be computed in vertex shader instead of compute shader? (Would simplify but might hit performance limit)
