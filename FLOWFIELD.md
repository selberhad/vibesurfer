# FLOWFIELD.md

## Concept: The Sound–Surface Feedback Loop

Skiwave’s ocean is not simulated water — it’s a living waveform.  
Every crest, trough, and shimmer is directly derived from the procedural music engine.  
The world *is* the soundtrack, and the soundtrack *is* the world.

---

## Core Idea

The **flowfield** is a shared substrate between:
1. Audio synthesis — generates procedural synthwave music.  
2. Wavefield simulation — renders the “liquid” terrain.  
3. Player feedback — feeds motion data back into the sound.

It operates as a continuous bidirectional loop:  
Music drives motion → motion modulates music → loop.

---

## 1. Wavefield Generation (Music → Terrain)

Each frame, the ocean’s geometry is updated from active audio buffers:

Frequency Band | Visual Effect | Behavior  
--------------- | -------------- | ----------  
Low (20–200 Hz) | Large-scale undulations | Drives the macro shape of the ocean (swells, rolls)  
Mid (200–2 kHz) | Choppiness, foam detail | Adds local turbulence and directional ripples  
High (2 k–10 kHz) | Specular sparkle, color modulation | Animates neon glints and glow intensity  

Height function example (in pseudocode):  
    height(x, z, t) = Σ(freq_bands)[ amplitude(f) * sin(k_f * (x + z) + phase(f, t)) ]  
All parameters (amplitude, phase, color) are read directly from the synth engine’s state — no randomization.

---

## 2. Motion Feedback (Player → Music)

The player is a live input modulator:

Player Action | Audio Effect | Visual Reinforcement  
-------------- | ------------- | --------------------  
Speed increases | Expanding stereo width, brighter timbre | Wider field of view, glow intensifies  
Jump / airtime | Temporary cutoff lift, harmonic bloom | Horizon flash, lens bloom spike  
Smooth carving | Adds phaser or chorus modulation | Waves align smoothly under board  
Chaotic input | Distortion and rhythmic gating | Turbulent wake trails  

Synth parameters (filter cutoff, delay feedback, envelope shape) respond to normalized motion metrics.

---

## 3. Flow Tracking Layer

A lightweight system continuously computes “flow”:  
    flow = sigmoid(smoothness * speed * timing)

Flow governs:  
- Player scoring  
- Audio energy levels  
- Visual palette transitions (cool → warm tones)

The higher the flow, the calmer and more melodic the world becomes.  
Lose rhythm, and dissonance and turbulence creep in.

---

## 4. Architecture Overview

System diagram (conceptual):

        Procedural Synthesizer
                  │
          Frequency Analysis
                  │
          Wavefield Generator  ←── Player motion feedback
                  │
            Render / Audio Mix

- Shared data bus carries per-frame signal summaries (amplitude envelopes, FFT bins).  
- No external sync: physics and audio clocks share a unified timestep.  
- Deterministic replay: identical inputs reproduce identical soundscapes.

---

## 5. Aesthetic Principle

Harmony through control, chaos through noise.  
The better you ride, the more the world sings.  
When you lose your rhythm, the sea itself protests.  
Every session becomes a procedural duet between code and player.

---

## Tagline

Flow is the instrument.