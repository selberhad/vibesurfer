# Toy 4: Spherical Chunk Streaming

## Purpose

Validate chunk-based terrain streaming on a sphere for infinite ocean navigation.

## Goals

1. **Performance testing** - Find vertex budget at 60 FPS
2. **Chunk streaming** - Validate async GPU chunk generation
3. **Orbital camera** - Test smooth circular navigation at any altitude
4. **LOD feasibility** - Determine if LOD is necessary

## Non-Goals

- Audio reactivity (defer to integration)
- Collision detection
- Physics simulation
- Realistic water shading

## Technical Approach

### Sphere Projection

**Grid to sphere mapping:**
```wgsl
// Local chunk coordinates (flat grid)
let local_x = f32(idx % CHUNK_SIZE) * spacing;
let local_z = f32(idx / CHUNK_SIZE) * spacing;

// Convert to lat/lon offset from chunk center
let lat = chunk.center_lat + (local_z - HALF_SIZE) / PLANET_RADIUS;
let lon = chunk.center_lon + (local_x - HALF_SIZE) / PLANET_RADIUS;

// Project to sphere surface
let r = PLANET_RADIUS + noise_height(lat, lon);
position = vec3(
    r * cos(lat) * cos(lon),
    r * sin(lat),
    r * cos(lat) * sin(lon)
);
```

### Orbital Camera

**Simplified orbital mechanics:**
```rust
// Position defined by angle around sphere
struct OrbitCamera {
    altitude: f32,         // Height above surface (meters)
    angular_pos: f32,      // Angle around sphere (radians)
    angular_velocity: f32, // Orbital speed (rad/s)
}

// Update: just increment angle
fn update(&mut self, dt: f32) {
    self.angular_pos += self.angular_velocity * dt;

    let r = PLANET_RADIUS + self.altitude;
    self.position = spherical_to_xyz(r, PI/2.0, self.angular_pos);
}
```

**No gravity, no acceleration - just pure circular motion**

### Chunk Streaming

**Phase 1: Single static chunk**
- Camera orbits, terrain is fixed
- Test different grid spacings (2m, 1m, 0.5m)
- Measure FPS vs vertex count

**Phase 2: 3-chunk streaming**
- Load chunk ahead of camera
- Unload chunk behind camera
- Measure streaming overhead

**Phase 3: Full 3×3 or 5×5 grid**
- Load all neighbors of current chunk
- Unload chunks outside visible radius
- Test with different chunk counts

### Chunk ID System

```rust
// Chunk identified by lat/lon grid cell
#[derive(Hash, Eq, PartialEq, Copy, Clone)]
struct ChunkId {
    lat_cell: i32,  // Which latitude band
    lon_cell: i32,  // Which longitude slice
}

impl ChunkId {
    fn from_latlon(lat: f32, lon: f32, chunk_size_radians: f32) -> Self {
        ChunkId {
            lat_cell: (lat / chunk_size_radians).floor() as i32,
            lon_cell: (lon / chunk_size_radians).floor() as i32,
        }
    }

    fn neighbors(&self) -> Vec<ChunkId> {
        // Return 8 neighboring chunks (3×3 grid)
    }
}
```

## Configurable Parameters

```rust
// Planet
const PLANET_RADIUS: f32 = 1_000_000.0;  // 1000km (smaller than Earth for testing)

// Orbit
const DEFAULT_ALTITUDE: f32 = 100.0;     // 100m above surface
const DEFAULT_SPEED: f32 = 100.0;        // 100 m/s

// Grid
const CHUNK_SIZE: u32 = 256;             // 256×256 vertices per chunk
const DEFAULT_SPACING: f32 = 2.0;        // 2m between vertices

// Streaming
const CHUNK_LOAD_RADIUS: u32 = 1;        // Load chunks within 1 cell (3×3 grid)
```

## Keyboard Controls

- `1` - Increase altitude (+10m)
- `2` - Decrease altitude (-10m)
- `3` - Increase speed (+10 m/s)
- `4` - Decrease speed (-10 m/s)
- `5` - Decrease grid spacing (more vertices)
- `6` - Increase grid spacing (fewer vertices)
- `7` - Toggle chunk streaming on/off
- `Space` - Pause/resume orbit
- `P` - Print performance stats

## Performance Targets

- **Minimum:** 60 FPS with 1M vertices (single large chunk)
- **Goal:** 60 FPS with 2-3M vertices (3×3 chunks)
- **Stretch:** 60 FPS with 5×5 chunks + LOD

## Success Criteria

1. Smooth orbital motion at any altitude (1m to 1000m)
2. No visible seams between chunks
3. Chunk streaming causes no perceptible stuttering
4. Can identify optimal chunk size and spacing
5. Clear performance profile (vertices vs FPS)

## Implementation Phases

### Phase 1: Single Chunk Orbit (1 hour)
- Sphere projection compute shader
- Orbital camera
- Wireframe rendering
- Grid spacing controls

### Phase 2: Chunk Streaming (1 hour)
- Chunk ID system
- Load/unload logic
- Async GPU compute
- Multi-chunk rendering

### Phase 3: Performance Tuning (30 min)
- Test different configurations
- Profile GPU compute time
- Document findings

## Deliverables

- Working prototype with configurable parameters
- Performance data (vertices vs FPS chart)
- LEARNINGS.md with findings
- Recommendation for Vibesurfer integration
