# Learnings: Validated Patterns and Constraints

Painful-to-discover patterns from toy implementations. No theory, just validated solutions.

---

## Framework Choice: Why Raw wgpu

**Bevy 0.17 has critical bugs that block Skiwave's requirements:**

1. **AlphaMode::Blend breaks custom material bindings**
   - Symptom: Shader compiles, runs, renders nothing (silent failure)
   - Works in `AlphaMode::Opaque`, fails in `AlphaMode::Blend`
   - Custom uniforms via `AsBindGroup` unavailable in blend pipeline
   - No workaround except hardcoding params in shader (dealbreaker for audio-reactive)

2. **Misleading API: `#[uniform()]` creates storage buffers**
   - Attribute named "uniform" actually generates `var<storage, read>`
   - Must use `var<storage, read>` in shader, not `var<uniform>`
   - Error message: "Storage class Storage doesn't match shader Uniform"
   - Cost: 2+ hours debugging counter-intuitive behavior

**Decision: Raw wgpu**
- Trade-off: +150 LOC boilerplate vs zero framework bugs
- What Skiwave needs: single mesh + custom shader + dynamic uniforms + alpha blend
- What Bevy offers: ECS, components, materials (don't need, and materials are broken)
- Verdict: 150 LOC is one-time cost, Bevy bugs are ongoing friction

---

## Raw wgpu Patterns

### Uniform Buffer Setup
```rust
let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Uniform Buffer"),
    contents: bytemuck::cast_slice(&[uniforms]),
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
});
```

### Dynamic Uniform Updates (Per-Frame)
```rust
queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
```

### Alpha Blending (Works Correctly)
```rust
fragment: Some(wgpu::FragmentState {
    targets: &[Some(wgpu::ColorTargetState {
        format: config.format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })],
}),
```

### Shader Uniforms (No Tricks)
```wgsl
struct Uniforms {
    view_proj: mat4x4<f32>,
    line_width: f32,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;  // Standard uniform, just works
```

---

## Audio Synthesis (Glicol + cpal)

### Block Size and Latency
- **Block size**: 128 samples
- **Sample rate**: 44100 Hz
- **Latency per block**: 2.9ms (128 / 44100)
- **Target**: <3ms for audio synthesis ✅
- **Measured**: <1ms for Glicol synthesis (plenty of headroom)

### Parameter Modulation via send_msg()
```rust
engine.send_msg(&format!("chain_name,chain_pos,param_pos,{}", value));
```
- **Latency**: Effectively zero (applies to next block = 2.9ms)
- **No parser overhead**: Bypasses text DSL entirely
- **Use case**: Runtime audio-reactive parameter modulation

### Audio Safety Limiter (Critical)
```rust
// Hard clip to ±0.5 to prevent ear damage
let left = buffers[0][i].clamp(-0.5, 0.5);
let right = buffers[1][i].clamp(-0.5, 0.5);
```
- **Why**: Synthesis params can go haywire during experiments
- **Lesson**: Even at "whisper quiet" multipliers (0.01), clipping at ±1.0 caused jet-engine volume
- **Solution**: Clamp well below ±1.0 threshold (±0.5 is safe)

---

## FFT Analysis

### Configuration
- **FFT size**: 1024 samples
- **Update rate**: 50ms (20 Hz, fast enough for reactivity)
- **Window function**: Hann window (reduces spectral leakage)
- **Overlap**: 50% (drain half buffer each cycle for smooth analysis)

### Frequency-to-Bin Mapping
```rust
fn hz_to_bin(hz: f32, fft_size: usize, sample_rate: usize) -> usize {
    ((hz * fft_size as f32) / sample_rate as f32) as usize
}
```
- **Bin resolution**: `sample_rate / fft_size` (44100 / 1024 ≈ 43 Hz per bin)
- **Don't hardcode bin ranges** (e.g., `1..10, 10..50`) - calculate from Hz

### Band Normalization (Essential)
```rust
let low: f32 = fft_output[bass_bins].iter()
    .map(|c| c.norm())
    .sum::<f32>() / bass_bins.len() as f32;  // Divide by bin count
```
- **Why**: Raw FFT magnitudes vary wildly between bands
- **Solution**: Average over bin count gives stable 0-10 range
- **Result**: Scales naturally to visual parameters

### Hann Window Function
```rust
fn hann_window(index: usize, size: usize) -> f32 {
    0.5 * (1.0 - ((2.0 * PI * index as f32) / (size as f32 - 1.0)).cos())
}
```

---

## Multi-Threading Architecture

### Three-Thread Pattern
```
Audio Thread (cpal callback, ~2.9ms)
  ├─> Glicol synthesis
  ├─> Accumulate samples to FFT buffer (Arc<Mutex<>>)
  └─> Output to speakers

FFT Thread (50ms loop)
  ├─> Read accumulated samples
  ├─> Apply Hann window + perform FFT
  └─> Extract low/mid/high bands → Arc<Mutex<AudioBands>>

Render Thread (16.67ms @ 60 FPS)
  ├─> Read FFT bands
  ├─> Map bands → ocean params (amplitude, frequency, line_width)
  ├─> Update mesh vertices (Perlin noise)
  └─> Render with wgpu
```

### Thread Coordination
- **Arc<Mutex<>>**: Simple, sufficient for 60 FPS
- **No contention observed** at this scale
- **Future optimization**: Lockless ring buffer (not needed yet)

---

## Audio-Reactive Mapping

### Validated Mappings
- **Bass (20-200 Hz) → Wave amplitude**: Natural, visceral (bass "hits" surge the ocean)
- **Mids (200-1000 Hz) → Wave detail/frequency**: More notes = more turbulence
- **Highs (1000-4000 Hz) → Line glow**: Subtle sparkle on harmonics

### Base + Modulation Pattern
```rust
let amplitude = base_amplitude + bands.low * scale_factor;
let frequency = base_frequency + bands.mid * scale_factor;
let line_width = base_line_width + bands.high * scale_factor;
```
- **Base values matter**: Too much modulation = seizure-inducing chaos
- **Sweet spot**: 2x-3x multiplier on base params
- **Scale factors discovered empirically**: `3.0` (bass), `0.15` (mid), `0.03` (high)

---

## Camera Journey Constraints

### Altitude Bounds (Critical)
- **Problem**: Ocean disappears when camera altitude exceeds ~300 units
- **Root cause**: At extreme altitude + horizontal view angle, ocean falls outside frustum
- **Solution**: Clamp altitude based on ocean size and look-at angle
- **Working params**: 50-110 unit altitude for 128×128 grid at 10-unit spacing

### Look-At Targeting
```rust
let target_y = camera_y * 0.7;  // Look slightly down toward ocean
```
- **Why 0.7**: At 1.0 (horizontal), camera looks over ocean at high altitude
- **Result**: Maintains visual contact with surface while showing horizon

---

## Performance Measurements (M1 Max)

### Ocean Rendering
- **Grid**: 128×128 = 16,641 vertices, 32,768 triangles
- **Per-frame work**: Update all vertices (Perlin noise) + write to GPU
- **Frame time**: ~8ms @ 60 FPS (12ms total budget used)
- **Headroom**: 30% (could handle larger grid or more complexity)

### Audio + FFT
- **Glicol synthesis**: <1ms (well under 3ms target)
- **FFT processing**: ~2ms on separate thread (non-blocking)
- **Zero audio dropouts** observed during development

---

## Glicol Composition Notes

### What Works
- Gate-triggered sequences (`speed 2.0 >> seq ...`)
- Envelope percussion (`envperc 0.001 0.1`)
- Filtered sawtooth lead (`saw >> lpf`)
- Modulating filters (`sin 0.2 >> mul 1300 >> add 1500`)
- `choose` randomization (melodic variation)
- Plate reverb (`plate 0.1`)

### Removed from Playground Examples
- JS pitch interpolation (`##Math.pow...#`) - not supported in engine mode
- Rhai meta scripts (`output.map(|x|x*0.1)`) - not needed
- Sample playback (`sp \808bd`) - requires audio file loading (procedural only)

### Volume Calibration
- **Initial volume**: `mul 0.1` was correct (comfortable listening)
- **Critical mistake**: Used real Hz (262, 196, 330) instead of MIDI-style values (48-72)
- **Result**: Sawtooth harmonics at high freq + reverb caused extreme clipping
- **Lesson**: Glicol expects MIDI-range note numbers or careful frequency scaling

---

## Architecture Decisions

### Module Separation
```
params.rs     - All magic numbers → typed structs with physical units
ocean.rs      - Ocean simulation + audio-reactive mapping
audio.rs      - Glicol engine + FFT analysis (isolated from rendering)
camera.rs     - Procedural camera journey (parameterized)
rendering.rs  - wgpu pipeline (isolated from simulation logic)
main.rs       - Minimal coordinator (<200 lines)
```

### Why This Separation
- **Tested independently**: Each module has unit tests
- **No hidden state**: All parameters in typed structs with docs
- **No magic numbers**: Every value has physical units and meaning
- **Reusable systems**: Camera/ocean/audio don't know about each other

---

## Constraints Documentation

When hitting performance/API/behavioral limits, document in this section.

### Example Format
```
**Constraint**: [What doesn't work]
**Symptom**: [How it fails]
**Root cause**: [Why it fails]
**Workaround**: [How to work within limit]
**Rationale**: [Why this constraint exists]
```

(None beyond those listed above yet)
