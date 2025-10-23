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

**Known issue: Black stripe artifacts at chunk boundaries**

**Root cause identified:** Sub-pixel gaps between independently-generated chunks
- Each chunk generates 256×256 grid with its own coordinate system
- Edge vertices in adjacent chunks calculate positions using different math (different chunk centers)
- Floating-point precision differences create tiny gaps (< 1 pixel)
- Gaps invisible with wireframe rendering (only edges visible)
- Gaps show as black stripes with solid surface rendering (background shows through)

**What DIDN'T work:**
1. ❌ Disabling backface culling - Stripes persist (not a culling issue)
2. ❌ Analytical sphere normals (`normalize(position)`) - Stripes persist (not a normal calculation issue)
3. ❌ Finite difference normals with neighbor access - Stripes persist (normals are consistent)
4. ❌ 0.5 cell grid offset for overlap - Stripes persist (chunks still use different centers)
5. ❌ Global grid alignment from chunk corner - Stripes persist (floating-point still differs)

**Why approaches failed:**
- Each chunk uses `params.chunk_center_lon` to calculate all positions
- Even with identical grid coordinates, different centers → different floating-point paths
- `chunk_corner = center - offset` still accumulates different rounding errors per chunk

**Potential solutions for future sessions:**

1. **Shared vertex buffer approach**
   - Create single global vertex buffer for all chunks
   - Each chunk references subset of vertices (no duplication at edges)
   - Requires: Global index mapping, more complex memory management
   - Benefit: Guaranteed identical edge vertices (same memory location)

2. **Explicit stitching with neighbor data**
   - Pass neighbor chunk centers to compute shader
   - Edge vertices query: "am I on boundary? Use neighbor's calculation"
   - Requires: ChunkId-aware compute shader, neighbor lookup
   - Benefit: Deterministic edge matching

3. **Integer-based grid coordinates**
   - Use global integer grid (lat_cell, lon_cell, grid_x, grid_z)
   - Convert to float only for final sphere projection
   - Minimizes floating-point accumulation differences
   - Benefit: Bitwise-identical edge calculations

4. **Accept artifacts, mask with rendering**
   - Slightly thicken wireframe at chunk boundaries to cover gaps
   - Or: use fog/depth bias to hide seams
   - Benefit: Simple, no architectural changes
   - Downside: Doesn't fix root cause

5. **Increase chunk overlap to 1+ vertices**
   - Generate 257×257 vertices (not 256×256)
   - Render only interior 255×255 (skip edges)
   - Neighboring chunks' interiors overlap by 1 vertex
   - Benefit: Redundant calculation guarantees coverage
   - Downside: 2% vertex overhead, z-fighting at overlaps

**Recommended next step:** Implement approach #1 (shared vertex buffer) - the architecturally correct solution that guarantees perfect edge matching through shared memory.

## Open Questions
- Does chunk grid need latitude expansion at higher latitudes? (Currently only longitude streaming)
- What's the minimum chunk resolution before terrain aliasing becomes visible?
- Can noise be computed in vertex shader instead of compute shader? (Would simplify but might hit performance limit)
- Would integer-based grid coordinates eliminate floating-point precision gaps?
