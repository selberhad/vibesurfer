# Vibesurfer

[![Built with DDD](https://img.shields.io/badge/built_with-DDD-blue)](https://github.com/dialecticianai/ddd-book)

> *Surf the sound.*

A fluid, retro-futuristic jet-surfing simulator where you glide, dive, and carve across an endless neon ocean. The surface behaves like living music: waves pulse to the beat, currents shimmer with color, and your motion becomes rhythm.

## What Makes It Unique

- **Procedural everything**: Ocean surface, music, lighting — all generated in real-time from code and math
- **Audio-reactive terrain**: Music frequency bands directly drive ocean geometry (bass → swells, mids → chop, highs → sparkle)
- **Zero static assets**: No pre-recorded audio, no 3D models, no textures — pure procedural generation
- **Infinite ocean**: Toroidal grid wrapping creates endless horizons as you surf

## System Requirements

- **Platform**: macOS Apple Silicon (M1/M2/M3)
- **Rust**: Stable toolchain (latest)
- **Graphics**: Metal-capable GPU (via wgpu)
- **Audio**: CoreAudio output device

## Quick Start

### Build and Run

```bash
# Clone the repository
git clone https://github.com/selberhad/vibesurfer.git
cd vibesurfer

# Build and run (optimized dev build)
cargo run --release

# Or run in default dev mode
cargo run
```

### Command-Line Options

```bash
# Fixed camera (default) - stationary view at specified elevation
cargo run -- --camera-preset fixed --elevation 101

# Basic camera - straight-line flight
cargo run -- --camera-preset basic

# Cinematic camera - procedural sweeping journey
cargo run -- --camera-preset cinematic

# Record gameplay to video (60fps)
cargo run -- --record 10  # 10 seconds
```

**Controls**:
- `ESC` - Quit

### Recording Output

When using `--record`, frames and audio are captured to `recording/`:
- `frames/` - Individual PNG frames
- `audio.wav` - Synchronized audio track
- `output.mp4` - Combined video (created by `scripts/combine-recording.sh`)

To combine frames into video:
```bash
./scripts/combine-recording.sh
```

## Project Structure

See [`CODE_MAP.md`](CODE_MAP.md) for detailed module documentation.

Key directories:
- `src/` - Rust source code (audio, camera, ocean, rendering)
- `scripts/` - Build automation and tooling
- `docs/` - Architecture and design documentation
- `recording/` - Video capture output (generated)

## Documentation

- **[VISION.md](VISION.md)** - Project vision and design philosophy
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and technical approach
- **[FLOWFIELD.md](FLOWFIELD.md)** - Audio-visual feedback loop concept
- **[CODE_MAP.md](CODE_MAP.md)** - Module-by-module code navigation
- **[CLAUDE.md](CLAUDE.md)** - AI development workflow guide
- **[LEARNINGS.md](LEARNINGS.md)** - Validated patterns and constraints

## Performance

**Target**: 60 FPS (16.67ms frame budget)

Measured on M1 Max:
- Ocean mesh: 512×512 grid (262,144 vertices, 524,288 triangles)
- Frame time: ~12ms (ocean update + rendering)
- Audio synthesis: <1ms per block (2.9ms @ 128 samples/44.1kHz)
- FFT analysis: ~2ms (separate thread, non-blocking)

## Development Philosophy

This project follows **Dialectic-Driven Development (DDD)** - a learning-driven workflow optimized for human-AI collaboration. See [`CLAUDE.md`](CLAUDE.md) for full methodology.

**Core principle**: Artifacts are disposable, clarity is durable. Code can be rewritten, insights cannot.

## Architecture Highlights

- **Three-thread model**: Audio synthesis, FFT analysis, rendering (see [ARCHITECTURE.md](ARCHITECTURE.md))
- **Two-layer terrain**: Stable base hills (100m amplitude) + audio-reactive detail (2m ripples)
- **Raw wgpu**: Direct graphics control (Bevy 0.17 has critical alpha blend bugs)
- **Glicol synthesis**: Procedural music engine with real-time modulation

## Contributing

This is a personal research project exploring procedural generation and audio-reactive systems. Not currently accepting contributions, but feel free to fork and experiment!

## License

See [LICENSE](LICENSE) for details.

## Links

- **Repository**: https://github.com/selberhad/vibesurfer
- **Vision**: See [VISION.md](VISION.md) for project goals and aesthetic targets
- **Technical Deep Dive**: See [ARCHITECTURE.md](ARCHITECTURE.md) for system design
