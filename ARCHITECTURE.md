# Architecture

High-level system design for Vibesurfer - an audio-reactive procedural ocean simulator.

---

## Design Philosophy

**Procedural everything** - No static assets. Ocean surface, music, lighting all generated from code and math.

**Audio-visual unity** - Music and terrain share the same underlying waveform parameters. The ocean *is* the soundtrack.

**Native-first** - High performance on Apple Silicon, with potential web fallback via WebGPU.

**Flow over realism** - Physics tuned for grace and feel, not simulation accuracy.

---

## System Overview

Vibesurfer uses a **three-thread architecture** to parallelize audio synthesis, FFT analysis, and rendering:

```
Audio Thread (cpal callback, ~2.9ms @ 128 samples)
  ├─> Glicol procedural synthesis
  ├─> Accumulate samples to FFT buffer (Arc<Mutex<>>)
  └─> Output to speakers (+ optional WAV recording)

FFT Thread (50ms loop)
  ├─> Read accumulated audio samples
  ├─> Apply Hann window + perform FFT (rustfft)
  └─> Extract low/mid/high bands → Arc<Mutex<AudioBands>>

Render Thread (16.67ms @ 60 FPS)
  ├─> Read FFT bands
  ├─> Map bands → ocean parameters (amplitude, frequency, line_width)
  ├─> Update mesh vertices (Perlin noise + toroidal wrapping)
  └─> Render with wgpu (+ optional frame capture)
```

### Thread Coordination

- **Arc<Mutex<>>**: Simple shared state, sufficient for 60 FPS (no contention observed)
- **No lockless structures yet**: Premature optimization — current approach works
- **Future consideration**: Ring buffer for FFT samples if contention emerges

---

## Core Subsystems

### 1. Audio System (`audio.rs`)

**Purpose**: Generate procedural music and extract frequency bands for visual reactivity.

**Components**:
- **Glicol engine**: Procedural synthesis using graph-based DSL
  - Composition: Gated sawtooth lead with envelope + reverb
  - Real-time parameter modulation via `send_msg()` (future: player input → synth params)
- **FFT analysis**: 1024-sample window, 50ms update rate, Hann windowing
  - Bass (20-200 Hz) → Large wave swells
  - Mids (200-1000 Hz) → Chop and turbulence
  - Highs (1000-4000 Hz) → Line glow and sparkle

**Key decisions**:
- **Block size**: 128 samples (2.9ms latency @ 44.1kHz)
- **Safety limiter**: Hard clip to ±0.5 (prevents ear damage during experiments)
- **Normalization**: Divide FFT magnitudes by bin count (stable 0-10 range)

### 2. Ocean System (`ocean.rs`)

**Purpose**: Procedural terrain generation with audio-reactive modulation.

**Architecture**:
- **Two-layer terrain model**:
  1. **Base terrain**: Stable large-scale hills (100m amplitude, time-independent)
     - Purpose: Physics surface for future skiing gameplay
     - Generated from Perlin noise with low frequency (0.003 cycles/m)
  2. **Detail layer**: Audio-reactive ripples (2m amplitude, animated)
     - Purpose: Visual interest and music synchronization
     - Modulated by FFT bands: `amplitude = base + bass * 3.0`

- **Infinite ocean illusion**:
  - **Toroidal grid wrapping**: 512×512 grid flows backward as camera moves forward
  - **Vertex repositioning**: When vertices exit behind camera, wrap to front
  - **Phantom line filtering**: Stretched triangles (from wrapping) excluded from rendering
  - **World-space noise sampling**: Absolute coordinates ensure seamless terrain

**Data flow**:
```
AudioBands → OceanSystem::update()
  ├─> Map bands to detail parameters (amplitude, frequency, line_width)
  ├─> OceanGrid::update()
  │   ├─> Flow vertices backward (simulate camera motion)
  │   ├─> Toroidal wrapping (X and Z axes)
  │   ├─> Sample base terrain (Perlin, time-independent)
  │   ├─> Sample detail layer (Perlin, animated)
  │   └─> Combine layers: height = base + detail
  └─> Filter stretched triangles (phantom line removal)
```

### 3. Camera System (`camera.rs`)

**Purpose**: Procedural camera paths for cinematic exploration.

**Presets**:
1. **Fixed** - Stationary camera with simulated grid flow (debugging)
2. **Basic** - Straight-line flight at constant altitude
3. **Cinematic** - Complex procedural journey:
   - X axis: Sweeping arcs (dual-frequency sine)
   - Z axis: Forward progression + weaving
   - Y axis: Altitude swoops (clamped to avoid terrain collision)
   - Look-at: Panning target ahead of camera

**Key constraint**:
- **Altitude bounds**: Camera must stay 50-110m for 512×512 grid visibility
- **Look-at angle**: 0.7 * camera altitude (prevents looking over ocean at high altitude)

### 4. Rendering System (`rendering.rs`)

**Purpose**: wgpu-based graphics pipeline with custom shaders.

**Pipeline**:
1. **Skybox pass**: Fullscreen procedural gradient (dusk horizon)
2. **Ocean pass**: Wireframe triangles with alpha blending
   - Vertex shader: MVP transform, pass UVs
   - Fragment shader: Neon glow, audio-reactive line width
   - Blend mode: `ALPHA_BLENDING` (critical: Bevy 0.17 breaks this)

**Rendering parameters** (audio-reactive):
- `amplitude`: Wave height (visual only, doesn't affect mesh geometry in shader)
- `frequency`: Spatial detail (higher = more turbulent)
- `line_width`: Wireframe thickness + glow intensity

**Frame capture** (recording mode):
- Copy framebuffer to staging buffer
- Map GPU memory to CPU
- Write PNG frames to `recording/frames/`

**Why raw wgpu, not Bevy?**

Bevy 0.17 has critical bugs that block our requirements:
1. **AlphaMode::Blend breaks custom material bindings** (silent rendering failure)
2. **Misleading API**: `#[uniform()]` generates storage buffers, not uniforms
3. **Trade-off**: +150 LOC boilerplate vs zero framework bugs

For a single mesh + custom shader + dynamic uniforms, raw wgpu is simpler and more reliable.

### 5. Parameters System (`params.rs`)

**Purpose**: Typed configuration with physical units and documentation.

**Structs**:
- `OceanPhysics`: Grid size, terrain amplitudes, frequencies (meters, Hz)
- `FFTConfig`: Sample rate, FFT size, frequency band ranges
- `AudioReactiveMapping`: Scale factors (bass→amplitude, mid→frequency, high→glow)
- `CameraPreset`: Journey parameters (altitude, speed, oscillation frequencies)
- `RenderConfig`: Window size, FOV, clipping planes
- `RecordingConfig`: Duration, output directory, FPS

**Philosophy**: No magic numbers in code. Every constant extracted, typed, and documented.

---

## Data Flow

### Audio → Visual Pipeline

```
1. Glicol Engine (Audio Thread)
   ↓ generates audio samples
2. FFT Buffer Accumulation (Audio Thread)
   ↓ Arc<Mutex<Vec<f32>>>
3. FFT Analysis (FFT Thread)
   ↓ extracts frequency bands
4. AudioBands (Shared State)
   ↓ Arc<Mutex<AudioBands>>
5. Ocean System (Render Thread)
   ↓ maps bands to parameters
6. Vertex Update (Render Thread)
   ↓ Perlin noise + band modulation
7. wgpu Rendering (Render Thread)
   └─> Screen output + optional frame capture
```

### Camera → Terrain Pipeline

```
1. Camera Position Update (procedural or fixed)
   ↓ Vec3 world position
2. Ocean Grid Vertex Flow
   ↓ vertices move opposite to camera delta
3. Toroidal Wrapping
   ↓ vertices repositioned when exiting bounds
4. World-Space Noise Sampling
   ↓ absolute coordinates → Perlin noise
5. Height Calculation
   └─> base terrain + audio-reactive detail
```

---

## Performance Budget

**Target**: 60 FPS (16.67ms frame budget)

**Measured on M1 Max**:

| Subsystem | Budget | Actual | Notes |
|-----------|--------|--------|-------|
| Ocean update | <5ms | ~4ms | 512×512 grid, Perlin noise sampling |
| Rendering | <8ms | ~8ms | wgpu draw calls, alpha blending |
| Audio synthesis | <3ms | <1ms | Glicol block generation (128 samples) |
| FFT analysis | N/A | ~2ms | Separate thread (non-blocking) |
| **Total** | **16ms** | **~12ms** | **30% headroom** |

**Optimization headroom**:
- Could support larger grid (1024×1024) or more complex shaders
- Audio synthesis has 3x safety margin
- FFT runs async (no impact on frame time)

---

## Key Constraints

### 1. Camera Altitude Bounds

**Problem**: Ocean disappears when camera altitude exceeds ~300m (for 128×128 grid).

**Root cause**: At extreme altitude + horizontal view angle, ocean falls outside frustum.

**Solution**: Clamp altitude based on grid size and look-at angle.
- Working params: 50-110m for 512×512 grid @ 2m spacing
- Look-at target: 0.7 * camera altitude (maintains visual contact with surface)

### 2. FFT Bin Resolution

**Constraint**: Frequency resolution = `sample_rate / fft_size` (44.1kHz / 1024 ≈ 43 Hz/bin)

**Implication**: Can't precisely isolate narrow frequency ranges (e.g., 440 Hz A note).

**Workaround**: Use broad bands (bass: 20-200 Hz, mid: 200-1000 Hz, high: 1000-4000 Hz).

### 3. Toroidal Wrapping Artifacts

**Problem**: Phantom lines from stretched triangles when vertices wrap.

**Solution**: Filter indices before rendering (exclude triangles with edges >10× grid spacing).

**Cost**: 2-5% of triangles culled (acceptable trade-off for seamless wrapping).

### 4. Alpha Blending + Custom Uniforms

**Bevy bug**: `AlphaMode::Blend` breaks custom material bindings (silent failure).

**Decision**: Use raw wgpu (150 LOC vs debugging framework bugs).

**Lesson**: For simple pipelines, frameworks add complexity without benefit.

---

## Design Decisions

### Why Procedural Synthesis (Not Pre-Recorded Audio)?

**Reasons**:
1. **Real-time modulation**: Player input can affect synth parameters (future feature)
2. **Zero assets**: No audio files to manage, version, or load
3. **Infinite variation**: Every playthrough generates unique music
4. **Audio-visual unity**: Synth parameters = terrain parameters (shared waveform substrate)

**Trade-offs**:
- Higher CPU usage (but <1ms, so negligible)
- Limited musical complexity (graph-based DSL vs DAW)
- Requires synthesis expertise (Glicol learning curve)

### Why Two-Layer Terrain?

**Problem**: Audio-reactive terrain looks amazing but makes skiing physics unstable (surface constantly morphs).

**Solution**: Separate stable base (physics) from reactive detail (visuals).

**Benefits**:
1. **Stable collision surface**: Player doesn't fall through morphing terrain
2. **Visual drama preserved**: Audio reactivity still visible as ripples
3. **Performance**: Base terrain computed once per vertex wrap, not per frame

### Why Toroidal Wrapping (Not Chunked Terrain)?

**Alternatives considered**:
1. **Chunked LOD**: Generate/destroy chunks as camera moves
2. **Static large grid**: 10,000×10,000 grid (too many vertices)
3. **Toroidal wrapping**: Fixed-size grid, vertices repositioned

**Decision**: Toroidal wrapping

**Reasons**:
- Constant memory footprint (512×512 = 262k vertices)
- No allocation/deallocation (no GC pauses)
- Simple implementation (50 LOC)
- Seamless horizon (no chunk pop-in)

**Trade-off**: Need phantom line filtering (but cheap).

---

## Future Architecture Considerations

### Multi-Crate Workspace

Currently monolithic (`src/*.rs`). Future modular structure:

```
vibesurfer-core/       # Platform-agnostic logic (no I/O)
  ├─ ocean simulation
  ├─ audio-reactive mapping
  └─ deterministic state

vibesurfer-native/     # Native rendering + audio
  ├─ wgpu rendering
  ├─ cpal audio
  └─ winit windowing

vibesurfer-web/        # WebGPU + WebAudio bindings
  ├─ wasm-bindgen
  └─ Same core logic
```

**Benefit**: Test core logic independently, reuse for web build.

### Player Input Integration

Audio system already has `send_msg()` for parameter modulation. Future:
- Player speed → stereo width
- Jump/airtime → harmonic bloom
- Smooth carving → phaser/chorus

### Flow Tracking System

Currently no player state. Future scoring mechanic:
```rust
flow = sigmoid(smoothness * speed * timing)
```
- High flow → calmer ocean, melodic music
- Low flow → turbulence, dissonance

---

## References

- [VISION.md](VISION.md) - Project goals and aesthetic targets
- [FLOWFIELD.md](FLOWFIELD.md) - Audio-visual feedback loop concept
- [LEARNINGS.md](LEARNINGS.md) - Validated patterns and constraints
- [CODE_MAP.md](CODE_MAP.md) - Module-by-module code navigation
