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

## Lighting & Surface Rendering

**Lit surface + wireframe overlay creates retro aesthetic**
- Dark teal surface (0.0, 0.15, 0.2) with directional lighting
- Cyan wireframe (0.0, 1.0, 1.0) overlaid using UV-based grid detection
- Lighting modulates both surface and wireframe for depth perception
- Wireframe uses `fract(uv * 256)` to match chunk resolution

**Compute shader normals required for seamless lighting**
- Fragment shader derivatives (`dpdx`/`dpdy`) fail at chunk boundaries
- Each chunk rendered separately → derivatives only see current triangle
- Solution: Calculate normals in compute shader using finite differences
- Access neighbor vertex positions for globally consistent normals
- One-sided differences at grid edges (forward/backward vs central)

**Vertex layout alignment matters for GPU**
- 48-byte struct (3 vec4s): position(vec3+pad) + uv(vec2+pad2) + normal(vec3+pad)
- WGSL requires proper padding for vec3 fields
- Vertex attribute offsets: position@0, uv@16, normal@32

**Performance unchanged with lighting**
- 120 FPS with lit surface + wireframe (same as wireframe-only)
- Normal calculation in compute shader adds zero overhead (computed once per chunk)
- Validates "GPU compute is effectively free" at this scale

**Chunk seam problem - SOLVED with integer-based grid coordinates**

**Root cause:** Sub-pixel gaps between independently-generated chunks
- Each chunk generated vertices using its own `chunk_center_lon` (float)
- Edge vertices in adjacent chunks used different floating-point calculation paths
- `chunk_corner = center - offset` accumulated different rounding errors per chunk
- Result: Tiny gaps (< 1 pixel) visible as black stripes with solid surfaces

**Solution: Integer-based global grid coordinates**
- Changed `SphereParams` to use `chunk_origin_lon_cell` / `chunk_origin_lat_cell` (i32)
- Each chunk calculates: `global_cell = chunk_origin_cell + local_vertex_index`
- Convert to world space meters: `world_pos = f32(global_cell) * grid_spacing`
- Then to spherical coordinates: `lon = world_pos / planet_radius`
- **Key:** All chunks use identical math for vertices at same global coordinate

**Why it works:**
- Edge vertex at global coordinate (255, 0) is ALWAYS calculated the same way
- No matter which chunk (lon_cell=0 or lon_cell=1), formula is identical
- Integer addition before float conversion eliminates precision differences
- Bitwise-identical positions guaranteed for shared edges

**Implementation:**
- `Chunk::create()` calculates `chunk_origin_lon_cell = lon_cell * (chunk_size - 1)`
- Compute shader: `global_x = params.chunk_origin_lon_cell + i32(gx)`
- Zero performance cost: int addition vs float multiply/add is negligible on GPU
- No architectural complexity: chunks remain independent

**Debug visualization:**
- Added `debug_chunk_boundaries` flag to `CameraUniforms` (u32: 0=off, 1=on)
- Red chunk borders render only when flag enabled
- Default: false (seamless surface)
- Easy to enable for debugging future streaming issues

**Result:**
- ✅ Seamless chunk boundaries - no visible gaps
- ✅ 120 FPS unchanged (zero performance impact)
- ✅ Clean, simple solution (no shared buffer complexity)
- ✅ Chunks remain fully independent (easy streaming)

## Wireframe Rendering Artifacts (UNRESOLVED)

**Problem:** Moiré/aliasing patterns visible at chunk boundaries, oscillating with camera position.

**Observations:**
- Artifact appears around chunk seams (confirmed with debug red boundaries)
- Oscillates with camera lateral movement (50m sine wave oscillation)
- Not related to chunk loading (happens with static 3×3 grid)
- Not related to fog, lighting, or backface culling
- Artifact "hovers around" the seam line based on viewing angle

**Root cause:** Fragment shader-based wireframe rendering suffers from precision/aliasing issues.

**Attempted fixes:**
1. ❌ Anti-aliased wireframe using `fwidth` and `smoothstep` - no improvement
2. ❌ Vertex attribute `grid_coord` to avoid per-fragment lat/lon calculation - still has artifacts
3. ❌ Direct integer grid cell coordinates (avoid lat/lon roundtrip) - marginal improvement at best
4. ❌ Coarser grid spacing (10m vs 2m) - just makes grid uglier, artifacts persist
5. ❌ Backface culling enabled/disabled - no effect

**Current implementation:**
- Integer grid cell indices stored in vertex attributes (`grid_coord`)
- Rasterizer interpolates these across triangles
- Fragment shader uses `fract(grid_coord)` to detect grid lines
- Smoothstep anti-aliasing with fwidth derivatives

**Hypothesis:** The fundamental issue is that fragment shader-based wireframe rendering using `fract()` and distance fields creates aliasing when:
- Grid coordinates are interpolated across triangles at chunk boundaries
- Camera viewing angle causes precision issues in interpolated values
- `fract()` amplifies tiny floating-point differences

**Possible solutions not yet tried:**
- Render wireframe as actual line geometry (LineList topology) - proper but requires separate render pass
- Accept that fragment-based wireframe has inherent limitations at certain viewing angles

**Status:** Documented as known issue. Wireframe is functional but has visual artifacts at chunk boundaries under certain camera angles. Not a blocker for gameplay.

## Camera Lateral Oscillation (for testing)

**Implementation:** Added lateral sine wave motion to test chunk boundary artifacts
- `OrbitCamera` now has `time` field
- Position calculates: `lat_offset = sin(time * 0.3Hz * 2π) * 50m / planet_radius`
- Creates smooth 50m side-to-side oscillation (one cycle every ~3 seconds)
- Helps diagnose artifacts that move with camera position vs chunk position

**Shared lib approach:**
- Both `main.rs` and `test_render.rs` use same `OrbitCamera` from lib
- `test_render` takes time values as input, calculates camera position from time
- Guarantees identical camera behavior across interactive and screenshot modes

## Open Questions
- Does chunk grid need latitude expansion at higher latitudes? (Currently only longitude streaming)
- What's the minimum chunk resolution before terrain aliasing becomes visible?
- Can noise be computed in vertex shader instead of compute shader? (Would simplify but might hit performance limit)
- Should wireframe be rendered as actual line geometry to eliminate artifacts?
