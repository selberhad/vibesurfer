# Code Map

Module-by-module navigation guide for Vibesurfer codebase.

---

## Directory Structure

```
vibesurfer/
├── src/
│   ├── main.rs           # Entry point, event loop, app state
│   ├── lib.rs            # Library exports
│   ├── cli.rs            # Command-line argument parsing
│   ├── camera.rs         # Procedural camera paths (fixed, basic, cinematic)
│   ├── rendering.rs      # wgpu pipeline (skybox + ocean wireframe)
│   │
│   ├── audio/
│   │   ├── mod.rs        # Re-exports
│   │   ├── system.rs     # AudioSystem with cpal integration
│   │   ├── fft.rs        # FFT analysis thread
│   │   └── synthesis.rs  # Glicol composition constant
│   │
│   ├── ocean/
│   │   ├── mod.rs        # Re-exports, AudioBands type
│   │   ├── mesh.rs       # OceanGrid with toroidal wrapping
│   │   └── system.rs     # OceanSystem with audio coordination
│   │
│   └── params/
│       ├── mod.rs        # Re-exports
│       ├── audio.rs      # FFTConfig, audio_constants
│       ├── camera.rs     # Camera presets and journey params
│       ├── ocean.rs      # OceanPhysics, AudioReactiveMapping
│       └── render.rs     # RenderConfig, RecordingConfig
│
├── scripts/
│   ├── combine-recording.sh         # Merge frames + audio → MP4
│   ├── generate-coverage-report.sh  # Run tests with coverage analysis
│   └── generate-loc-report.sh       # Count lines of code (Rust + docs)
│
├── recording/                        # Video capture output (generated)
│   ├── frames/                       # PNG frames (numbered)
│   ├── audio.wav                     # Synchronized audio
│   └── output.mp4                    # Combined video
│
├── .cargo/
│   └── config.toml                   # Cargo build configuration
│
├── Cargo.toml                        # Package manifest + dependencies
├── Cargo.lock                        # Dependency lock file
├── rustfmt.toml                      # Code formatting rules
├── .gitignore                        # Git ignore patterns
│
├── README.md                         # Project overview + quick start
├── ARCHITECTURE.md                   # System design deep dive
├── CODE_MAP.md                       # This file (module navigation)
├── VISION.md                         # Project goals and aesthetic targets
├── FLOWFIELD.md                      # Audio-visual feedback loop concept
├── CLAUDE.md                         # AI development workflow guide
├── LEARNINGS.md                      # Validated patterns and constraints
├── LEXICON.md                        # DDD methodology terminology
├── LOC_REPORT.md                     # Lines of code metrics (generated)
└── COVERAGE_REPORT.md                # Test coverage report (generated)
```

---

## Module Documentation

### `src/main.rs` - Application Entry Point

**Purpose**: Winit event loop, app state management, frame coordination.

**Key types**:
- `Args` - Command-line argument parser (clap)
  - `--record SECONDS` - Enable video capture mode
  - `--camera-preset PRESET` - Select camera path (fixed, basic, cinematic)
  - `--elevation METERS` - Fixed camera altitude
- `App` - Main application state
  - `window: Arc<Window>` - Winit window handle
  - `render_system: RenderSystem` - wgpu pipeline
  - `ocean: OceanSystem` - Procedural terrain
  - `camera: CameraSystem` - Camera path generator
  - `audio: AudioSystem` - Synthesis + FFT
  - `frame_count` - Frame counter for recording

**Flow**:
1. Parse CLI args → `Args`
2. Create `App` with selected camera preset + recording config
3. Run winit event loop (`EventLoop::run_app`)
4. On `RedrawRequested`:
   - Get elapsed time
   - Read audio bands from FFT
   - Update camera position
   - Update ocean mesh (with audio modulation)
   - Render frame (+ capture if recording)
   - Exit when recording complete or ESC pressed

**Integration points**:
- Calls `camera.create_view_proj_matrix()` → Mat4
- Calls `ocean.update()` → (amplitude, frequency, line_width)
- Calls `render_system.render()` → frame output

---

### `src/lib.rs` - Library Exports

**Purpose**: Expose public module API for potential library use.

**Exports**:
- `pub mod audio` - Audio synthesis + FFT
- `pub mod camera` - Camera system
- `pub mod ocean` - Ocean simulation
- `pub mod params` - Configuration structs
- `pub mod rendering` - wgpu rendering

**Note**: Currently just a thin export layer. Future multi-crate workspace would use this as `vibesurfer-core`.

---

### `src/cli.rs` - Command-Line Argument Parsing

**Purpose**: Parse and process command-line arguments for camera and recording configuration.

**Key types**:
- `Args` - CLI argument struct (clap Parser)
  - `record: Option<f32>` - Recording duration
  - `camera_preset: String` - Camera mode selection
  - `elevation: f32` - Fixed camera altitude

**Functions**:
- `Args::parse_camera_preset()` - Convert CLI arg to CameraPreset enum
- `Args::create_recording_config()` - Setup recording directories and config

**Integration points**:
- Called by `main.rs` during startup
- Returns CameraPreset and optional RecordingConfig

---

### `src/audio/` Module - Audio Synthesis + FFT Analysis

**Purpose**: Generate procedural music and extract frequency bands for visual reactivity.

#### `src/audio/mod.rs` - Module Re-exports

**Purpose**: Public API for audio module.

**Exports**:
- `AudioSystem` from system.rs

#### `src/audio/system.rs` - Audio System Coordinator

**Key types**:
- `AudioSystem` - Main audio coordinator
  - `audio_bands: Arc<Mutex<AudioBands>>` - Shared FFT results
  - `_stream: cpal::Stream` - Audio output (kept alive)
  - `_fft_thread: JoinHandle<()>` - FFT analysis thread

**Functions**:
- `AudioSystem::new(fft_config, recording_config)` - Initialize audio + FFT threads
  - Creates Glicol engine
  - Spawns cpal output stream (audio callback)
  - Spawns FFT analysis thread
  - Optionally creates WAV writer for recording
- `AudioSystem::get_bands()` - Read current FFT bands (thread-safe)

**Audio callback flow** (runs on audio thread):
1. Lock Glicol engine
2. Generate audio blocks (`engine.next_block()`)
3. Fill output buffer (stereo interleaved)
4. Accumulate samples to FFT buffer
5. Write to WAV if recording

**Integration points**:
- Called by `main.rs` during app init
- Reads `AudioBands` via `get_bands()`

**Gotchas**:
- Hard clip to ±0.5 (safety limiter, prevents ear damage)
- Must fill entire cpal buffer (choppy audio if partial)

#### `src/audio/fft.rs` - FFT Analysis Thread

**Purpose**: Background thread for real-time frequency analysis.

**Functions**:
- `spawn_fft_thread(config, fft_buffer, audio_bands)` - Launch FFT analysis loop
  - Reads accumulated audio samples
  - Applies Hann window
  - Performs FFT (rustfft)
  - Extracts bass/mid/high bands with normalization
- `hann_window(index, size)` - Hann window function for FFT

**FFT thread flow** (runs every 50ms):
1. Check if FFT buffer has ≥1024 samples
2. Apply Hann window
3. Perform FFT
4. Extract frequency bands (normalized by bin count)
5. Update shared `AudioBands`
6. Drain 50% of buffer (overlap)

**Gotchas**:
- FFT bin resolution: 44.1kHz / 1024 ≈ 43 Hz/bin
- Must normalize by bin count for stable visual parameters

#### `src/audio/synthesis.rs` - Glicol Composition

**Purpose**: Procedural music synthesis configuration.

**Constants**:
- `GLICOL_COMPOSITION` - Procedural music DSL code
  - Gated sawtooth lead with envelope + reverb
  - Randomized note selection via `choose`

---

### `src/camera.rs` - Procedural Camera Paths

**Purpose**: Generate camera position and look-at target for each frame.

**Key types**:
- `CameraSystem` - Camera state manager
  - `preset: CameraPreset` - Active camera path
- `CameraPreset` - Enum of camera modes
  - `Fixed(FixedCamera)` - Stationary camera (debugging)
  - `Basic(BasicCameraPath)` - Straight-line flight
  - `Cinematic(CameraJourney)` - Complex procedural journey

**Functions**:
- `CameraSystem::new(preset)` - Create camera with selected preset
- `CameraSystem::create_view_proj_matrix(time, config)` - Generate view-projection matrix
  - Returns `(Mat4, Vec3)` - MVP matrix + camera position
  - Calls preset-specific position generator
- `CameraSystem::get_simulated_velocity()` - For fixed camera, returns velocity to flow grid
- `create_fixed_camera(...)` - Stationary view with simulated grid flow
- `create_basic_camera(...)` - Straight-line forward flight
- `create_cinematic_camera(...)` - Procedural journey with sweeping arcs

**Cinematic camera components** (all dual-frequency sine oscillations):
- **X axis**: Wide arcs (0.2 Hz primary + 0.7 Hz secondary)
- **Z axis**: Forward progression (constant speed) + weaving (0.5 Hz + 1.1 Hz)
- **Y axis**: Altitude swoops (0.3 Hz + 1.3 Hz), clamped to 50-110m
- **Look-at**: Panning target ahead (0.4 Hz X, 0.6 Hz Z, 0.5 Hz Y)

**Integration points**:
- Called by `main.rs` each frame
- Camera position passed to `ocean.update()` for grid flow

**Gotchas**:
- Altitude must stay 50-110m for 512×512 grid (ocean disappears if too high)
- Look-at target = 0.7 * camera Y (prevents looking over ocean)

---

### `src/ocean/` Module - Two-Layer Procedural Terrain

**Purpose**: Generate infinite ocean surface with stable base terrain + audio-reactive detail.

#### `src/ocean/mod.rs` - Module Re-exports

**Purpose**: Public API for ocean module and shared types.

**Types**:
- `AudioBands` - FFT frequency band energies (shared with audio module)
  - `low: f32` - Bass (20-200 Hz)
  - `mid: f32` - Mids (200-1000 Hz)
  - `high: f32` - Highs (1000-4000 Hz)

**Exports**:
- `Vertex`, `OceanGrid` from mesh.rs
- `OceanSystem` from system.rs

#### `src/ocean/mesh.rs` - Ocean Grid Mesh

**Purpose**: Low-level mesh management with toroidal wrapping and Perlin noise.

**Key types**:
- `Vertex` - Mesh vertex data (`#[repr(C)]`, GPU-compatible)
  - `position: [f32; 3]` - World position
  - `uv: [f32; 2]` - Texture coordinates (unused currently)
- `OceanGrid` - Mesh with procedural noise animation
  - `vertices: Vec<Vertex>` - Mesh vertices (position + UV)
  - `indices: Vec<u32>` - Triangle indices (original)
  - `filtered_indices: Vec<u32>` - Indices after phantom line removal
  - `perlin: Perlin` - Noise generator (seeded)
  - `last_camera_pos: Vec3` - For computing delta movement
  - `base_terrain_heights: Vec<f32>` - Cached stable terrain (future physics use)

**Functions**:
- `OceanGrid::new(physics)` - Create mesh + noise generator
  - Generates flat XZ grid (512×512 = 262k vertices)
  - Generates triangle indices (counter-clockwise winding)
- `OceanGrid::update(time, detail_amplitude, detail_frequency, camera_pos, physics)`
  - **Step 1**: Compute camera delta (how much camera moved this frame)
  - **Step 2**: Flow vertices backward (opposite to camera motion)
  - **Step 3**: Toroidal wrapping (X and Z axes)
    - If vertex exits behind camera, wrap to front
    - Maintains seamless infinite ocean illusion
  - **Step 4**: Sample base terrain (Perlin, time-independent)
    - Large hills (100m amplitude, 0.003 frequency)
    - Stable physics surface for future skiing
  - **Step 5**: Sample detail layer (Perlin, animated)
    - Audio-reactive ripples (2m base amplitude + FFT modulation)
  - **Step 6**: Combine layers: `height = base + detail`
  - **Step 7**: Filter stretched triangles (phantom line removal)
- `OceanGrid::filter_stretched_triangles()` - Remove wrapped triangle artifacts
  - Excludes triangles with any edge >10× grid spacing
  - Prevents phantom lines from toroidal wrapping
- `OceanGrid::query_base_terrain(world_x, world_z, physics)` - Query stable terrain height
  - Future use: player collision detection

**Integration points**:
- Created by `OceanSystem::new()`
- Updated by `OceanSystem::update()` each frame

**Gotchas**:
- Must use absolute world coordinates for Perlin sampling (not grid-relative)
- Toroidal wrapping creates phantom lines (mitigated by filtering)
- Base terrain cached but not used yet (reserved for future physics)

#### `src/ocean/system.rs` - Ocean System Coordinator

**Purpose**: High-level ocean coordination with audio-reactive modulation.

**Key types**:
- `OceanSystem` - Ocean coordinator
  - `grid: OceanGrid` - Mesh + Perlin noise generator
  - `physics: OceanPhysics` - Configuration (from params module)
  - `mapping: AudioReactiveMapping` - FFT → visual parameter mapping

**Functions**:
- `OceanSystem::new(physics, mapping)` - Create ocean with configuration
- `OceanSystem::update(time, audio_bands, camera_pos)`
  - Maps audio bands to detail parameters:
    - `amplitude = base + bass * 3.0`
    - `frequency = base + mid * 0.15`
    - `line_width = base + high * 0.03`
  - Calls `grid.update()` to recompute mesh
  - Returns `(amplitude, frequency, line_width)` for shader uniforms

**Two-layer model**:
| Layer | Amplitude | Frequency | Time-dependent? | Purpose |
|-------|-----------|-----------|-----------------|---------|
| Base terrain | 100m | 0.003 | No (static hills) | Physics surface |
| Detail layer | 2m + audio | 0.1 + audio | Yes (animated) | Visual reactivity |

**Integration points**:
- Called by `main.rs` each frame with `AudioBands` + camera position
- Returns parameters for rendering.rs shader uniforms

---

### `src/params/` Module - Typed Configuration

**Purpose**: Extract all magic numbers into typed structs with physical units and documentation.

**Philosophy**: No bare constants in code. Every value has units, meaning, and rationale.

#### `src/params/mod.rs` - Module Re-exports

**Purpose**: Public API for params module.

**Exports**:
- `FFTConfig`, `audio_constants` from audio.rs
- `CameraPreset`, `CameraJourney`, `BasicCameraPath`, `FixedCamera` from camera.rs
- `OceanPhysics`, `AudioReactiveMapping` from ocean.rs
- `RenderConfig`, `RecordingConfig` from render.rs

#### `src/params/ocean.rs` - Ocean Parameters

**Purpose**: Ocean simulation physics and audio-reactive mapping configuration.

**Key types**:
- `OceanPhysics` - Ocean simulation parameters (~88 lines)
  - Grid dimensions (size, spacing)
  - Base terrain (amplitude, frequency)
  - Detail layer (amplitude, frequency)
  - Noise seed
- `AudioReactiveMapping` - FFT → visual parameter mapping
  - `bass_to_amplitude_scale: 3.0`
  - `mid_to_frequency_scale: 0.15`
  - `high_to_glow_scale: 0.03`

#### `src/params/audio.rs` - Audio Parameters

**Purpose**: FFT analysis configuration and audio constants.

**Key types**:
- `FFTConfig` - FFT analysis configuration (~94 lines)
  - Sample rate, FFT size, update interval
  - Frequency band ranges (bass, mid, high)
  - Helper methods: `hz_to_bin()`, `bass_bins()`, `validate()`

**Module constants**:
- `audio_constants::BLOCK_SIZE` - 128 samples (matches Glicol engine)

#### `src/params/camera.rs` - Camera Parameters

**Purpose**: Camera path configuration and presets.

**Key types**:
- `CameraPreset` - Enum of camera modes (~204 lines)
  - `Fixed(FixedCamera)` - Position, target, simulated velocity
  - `Basic(BasicCameraPath)` - Altitude, speed, look-ahead
  - `Cinematic(CameraJourney)` - Oscillation frequencies + amplitudes (many fields)

#### `src/params/render.rs` - Render Parameters

**Purpose**: Rendering and recording configuration.

**Key types**:
- `RenderConfig` - Window size, FOV, clipping planes (~84 lines)
  - Helper: `aspect_ratio()`
- `RecordingConfig` - Duration, output directory, FPS
  - Helper methods: `total_frames()`, `frames_dir()`, `audio_path()`

**Integration points**:
- Used by all modules to configure behavior
- Passed as arguments (no global state)

---

### `src/rendering.rs` - wgpu Graphics Pipeline

**Purpose**: Raw wgpu rendering with skybox + ocean wireframe.

**Key types**:
- `RenderSystem` - Main rendering coordinator
  - wgpu resources: device, queue, surface, pipeline, buffers
  - Vertex/index buffers (dynamic, updated per frame)
  - Uniform buffers (ocean + skybox)
  - Optional recording state (staging buffer, PNG encoder)
- `Uniforms` - Ocean shader uniforms (`#[repr(C)]`)
  - `view_proj: [[f32; 4]; 4]` - MVP matrix
  - `line_width: f32` - Wireframe thickness + glow
  - `amplitude: f32` - Wave height (visual only)
  - `frequency: f32` - Spatial detail
  - `time: f32` - Animation time
- `SkyboxUniforms` - Skybox shader uniforms
  - `inv_view_proj: [[f32; 4]; 4]` - Inverse MVP (for fullscreen raycast)
  - `time: f32` - Animation time
- `RecordingState` - Frame capture resources
  - Staging buffer (GPU → CPU)
  - PNG encoder thread

**Functions**:
- `RenderSystem::new(window, grid, recording_config)` - Initialize wgpu pipeline (async)
  - Creates device, queue, surface, swap chain
  - Loads + compiles shaders (ocean.wgsl, skybox.wgsl)
  - Creates render pipelines (skybox opaque + ocean alpha blend)
  - Creates buffers (vertex, index, uniform)
  - Sets up recording state if needed
- `RenderSystem::update_vertices(vertices)` - Upload new vertex data to GPU
- `RenderSystem::update_indices(indices)` - Upload new index data to GPU
- `RenderSystem::update_uniforms(uniforms)` - Update ocean shader uniforms
- `RenderSystem::update_skybox_uniforms(uniforms)` - Update skybox shader uniforms
- `RenderSystem::render(frame_count, index_count)` - Execute render passes
  - Acquire swap chain texture
  - **Skybox pass**: Fullscreen quad, procedural gradient
  - **Ocean pass**: Indexed draw, wireframe triangles, alpha blending
  - **Frame capture** (if recording): Copy to staging buffer, write PNG

**Shaders** (embedded in rendering.rs):
- `ocean.wgsl` - Vertex + fragment shader for ocean mesh
  - Vertex: MVP transform, pass UVs
  - Fragment: Neon glow based on line width + time
- `skybox.wgsl` - Fullscreen procedural skybox
  - Vertex: Fullscreen triangle trick (no vertex buffer)
  - Fragment: Dusk gradient (violet → orange horizon)

**Render pipeline config**:
- Primitive topology: `TriangleList`
- Polygon mode: `Fill` (wireframe effect done in shader, not rasterizer)
- Cull mode: `None` (see both sides of triangles)
- Blend mode: `ALPHA_BLENDING` (critical: Bevy 0.17 breaks this)
- Depth/stencil: None (skybox behind, ocean in front)

**Frame capture flow** (recording mode):
1. Render to swap chain texture as usual
2. Copy framebuffer to staging buffer (`copy_texture_to_buffer`)
3. Map staging buffer to CPU (async, but we wait)
4. Encode PNG on background thread
5. Write to `recording/frames/frameN.png`

**Integration points**:
- Called by `main.rs` each frame
- Receives vertices, indices, uniforms from `ocean.rs`
- Receives camera MVP from `camera.rs`

**Gotchas**:
- Must acquire/present swap chain texture each frame
- Staging buffer copy adds ~1ms latency (but unavoidable for capture)
- Shaders compiled at runtime (error checking at startup)

---

## Entry Points

### Application Startup

```
main() in main.rs
  ├─> Parse CLI args (clap)
  ├─> Create RecordingConfig (if --record)
  ├─> Parse CameraPreset (--camera-preset, --elevation)
  ├─> Create App struct
  │   ├─> OceanSystem::new(physics, mapping)
  │   ├─> CameraSystem::new(preset)
  │   └─> (RenderSystem + AudioSystem created in resumed())
  └─> Run EventLoop::run_app(app)
```

### Per-Frame Update

```
App::render_frame()
  ├─> Get elapsed time
  ├─> audio.get_bands() → AudioBands
  ├─> camera.create_view_proj_matrix(time) → (Mat4, Vec3)
  ├─> ocean.update(time, bands, camera_pos) → (amplitude, frequency, line_width)
  ├─> render_system.update_vertices(&ocean.grid.vertices)
  ├─> render_system.update_indices(&ocean.grid.filtered_indices)
  ├─> render_system.update_uniforms(Uniforms { ... })
  ├─> render_system.update_skybox_uniforms(SkyboxUniforms { ... })
  └─> render_system.render(frame_count, index_count)
```

---

## Testing

**Unit tests** (colocated in modules):
- `audio.rs`: FFT config validation, Hann window, bin mapping
- `ocean.rs`: Grid creation, audio-reactive mapping
- `params.rs`: FFT config validation, Hz-to-bin conversion

**Integration tests**: None yet (future: end-to-end rendering tests)

**Benchmarks**: None yet (future: criterion benchmarks for hot paths)

**Run tests**:
```bash
cargo test          # All tests
cargo test audio    # Audio module only
```

---

## Build Configuration

### Cargo.toml

**Dependencies**:
- `wgpu 23` - Graphics API (Metal on macOS)
- `winit 0.30` - Windowing + event loop
- `glicol 0.13` - Procedural synthesis engine
- `cpal 0.15` - Cross-platform audio output
- `rustfft 6` - FFT analysis
- `bytemuck 1.14` - Zero-copy type casting (vertex buffers)
- `noise 0.9` - Perlin noise generator
- `glam 0.29` - Linear algebra (Vec3, Mat4)
- `pollster 0.3` - Async executor (for wgpu init)
- `clap 4.5` - CLI argument parsing
- `hound 3.5` - WAV file writing (recording mode)
- `image 0.25` - PNG encoding (frame capture)

**Dev profile** (faster compile times):
- `opt-level = 1` - Light optimization for project code
- `opt-level = 3` (dependencies) - Full optimization for libraries

### .cargo/config.toml

Platform-specific build configuration (if present).

### rustfmt.toml

Code formatting rules (default Rust style).

---

## Scripts

### `scripts/generate-loc-report.sh`

**Purpose**: Count lines of code (Rust + documentation).

**Output**: `LOC_REPORT.md` (git-tracked)

**Metrics**:
- Rust code lines (excluding comments/blanks)
- Documentation lines (*.md files)
- Docs-to-code ratio

**Usage**: Runs automatically in pre-commit hook.

### `scripts/generate-coverage-report.sh`

**Purpose**: Run tests with coverage analysis (tarpaulin).

**Output**: `COVERAGE_REPORT.md` (git-tracked)

**Usage**:
```bash
ENABLE_COVERAGE=true ./scripts/generate-coverage-report.sh
```

**Note**: Disabled by default (slow). Enable with env var.

### `scripts/combine-recording.sh`

**Purpose**: Merge PNG frames + audio.wav → output.mp4

**Requirements**: `ffmpeg` installed

**Usage**:
```bash
./scripts/combine-recording.sh
```

**Input**: `recording/frames/*.png` + `recording/audio.wav`
**Output**: `recording/output.mp4` (60fps, H.264)

---

## Module Dependencies

```
main.rs
  ├─> audio.rs
  │     ├─> params.rs (FFTConfig, RecordingConfig, BLOCK_SIZE)
  │     └─> ocean.rs (AudioBands)
  ├─> camera.rs
  │     └─> params.rs (CameraPreset, RenderConfig)
  ├─> ocean.rs
  │     └─> params.rs (OceanPhysics, AudioReactiveMapping)
  └─> rendering.rs
        └─> ocean.rs (Vertex)

lib.rs (re-exports all modules)
```

**Dependency rules**:
- `params.rs` has no dependencies (pure data)
- `audio.rs` and `ocean.rs` share `AudioBands` type
- `main.rs` orchestrates all subsystems

---

## Gotchas and Anti-Patterns

### Don't: Hardcode Magic Numbers

**Bad**:
```rust
let amplitude = 2.0 + bass * 3.0;
```

**Good**:
```rust
let amplitude = physics.detail_amplitude_m
    + audio_bands.low * mapping.bass_to_amplitude_scale;
```

**Reason**: Every constant should be in `params.rs` with units and docs.

### Don't: Use Grid-Relative Coordinates for Noise

**Bad**:
```rust
let noise = perlin.get([vertex.position[0], vertex.position[2], t]);
```

**Good**:
```rust
let x_world = camera_pos.x + vertex.position[0];
let z_world = camera_pos.z + vertex.position[2];
let noise = perlin.get([x_world, z_world, t]);
```

**Reason**: Toroidal wrapping requires absolute world coordinates for seamless terrain.

### Don't: Forget Safety Limiter in Audio Callback

**Bad**:
```rust
data[i] = left;  // Can clip to ±1.0 = ear damage
```

**Good**:
```rust
data[i] = left.clamp(-0.5, 0.5);  // Safe limit
```

**Reason**: Synthesis bugs can produce extreme values. Protect your ears!

### Don't: Render All Triangles After Wrapping

**Bad**:
```rust
render_system.render(index_count);  // Includes phantom lines
```

**Good**:
```rust
grid.filter_stretched_triangles();
render_system.render(filtered_indices.len());
```

**Reason**: Wrapped vertices create stretched triangles (phantom lines).

---

## References

- [README.md](README.md) - Project overview + quick start
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design deep dive
- [VISION.md](VISION.md) - Project goals and aesthetic targets
- [LEARNINGS.md](LEARNINGS.md) - Validated patterns and constraints
