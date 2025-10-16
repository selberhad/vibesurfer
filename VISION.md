# VISION.md

## Project: Skiwave

### Core Idea
A fluid, retro-futuristic jet-surfing simulator — glide, dive, and carve across an endless neon ocean.  
The surface behaves like living music: waves pulse to the beat, currents shimmer with color, and your motion becomes rhythm.

### Goal
Deliver a native-first, procedural experience that captures pure flow.  
Every run should feel like surfing through sound and light — half game, half instrument.

### Guiding Principles
- **Native-first, web-graceful**: High performance natively, elegant fallback in browser.  
- **Procedural everything**: Ocean surface, waves, lighting, and sound all generated in real time.  
- **Flow over realism**: Physics tuned for grace, not accuracy.  
- **Music as terrain**: Waveforms literally shape the ride.  
- **Zero assets**: Code, color, and math only — no static art.  
- **Toy-first design**: Simple core loop, infinite replayability.

### Core Subsystems
- **Ocean Engine** — Procedural wavefield using hybrid noise and FFT synthesis.  
- **Surf Physics** — Jet-assisted movement with carve, pump, and air-trick dynamics.  
- **Avatar** — Minimalist figure or hoverboard silhouette, fluid IK motion.  
- **Audio Synthesis** — Gameplay-linked procedural music; your maneuvers modulate the mix.  
- **FX System** — Wireframe water, refraction, and glow tuned by “vibe” parameters.  
- **Flow Tracker** — Converts motion, amplitude, and timing into score and sound triggers.  
- **Camera** — Horizon-anchored cinematic drift with zoomable depth.

### Aesthetic Targets
- **Visual**: Endless dusk sea, violet horizon, waves of neon glass.  
- **Audio**: Dynamic synthwave evolving with player rhythm.  
- **Mood**: Meditative speed — calm immersion, not chaos.

### Architecture Intent
- Modular workspace:
  - `skiwave-core`: deterministic logic, ECS, flow systems.
  - `skiwave-native`: full 3D, advanced shaders, real-time synth.
  - `skiwave-web`: simplified WebGPU version, same logic.
- Procedural surfaces built from harmonic noise fields and signed-distance deformation.
- Audio and physics unified through shared waveform parameters (“the sea is the sound”).

### Success Criteria
- Instant sense of glide and musical feedback.  
- Feels alive — terrain responds like liquid energy.  
- Procedural music and motion remain in sync.  
- Web version preserves core flow and glow.  
- Nothing pre-rendered; every run unique.

### Tagline
> *Skiwave — surf the sound.*