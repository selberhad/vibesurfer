//! Ocean simulation physics parameters and audio-reactive mapping.

/// Ocean simulation physics parameters
#[derive(Debug, Clone)]
pub struct OceanPhysics {
    /// Grid resolution (vertices per side, e.g., 128 = 16,641 vertices)
    pub grid_size: usize,

    /// Spacing between grid vertices in world units (meters)
    pub grid_spacing_m: f32,

    /// Wave animation speed multiplier (dimensionless, affects time scaling)
    pub wave_speed: f32,

    // === Base terrain (stable physics surface for skiing) ===
    /// Base terrain amplitude in meters (large stable hills)
    pub base_terrain_amplitude_m: f32,

    /// Base terrain frequency (cycles per meter, low = long slopes)
    pub base_terrain_frequency: f32,

    // === Detail layer (audio-reactive visual ripples) ===
    /// Detail wave height in meters (before audio modulation)
    pub detail_amplitude_m: f32,

    /// Detail spatial frequency (cycles per meter, controls wave chop)
    pub detail_frequency: f32,

    /// Base wireframe line width (screen-space or shader units)
    pub base_line_width: f32,

    /// Perlin noise seed
    pub noise_seed: u32,
}

impl Default for OceanPhysics {
    fn default() -> Self {
        Self {
            grid_size: 512,      // Large enough for good view distance without lag
            grid_spacing_m: 2.0, // Fine spacing for many lines
            wave_speed: 0.5,

            // Base terrain: EXTREME Tribes-style hills for skiing (100m tall, long slopes)
            base_terrain_amplitude_m: 100.0,
            base_terrain_frequency: 0.003, // Even longer wavelengths for massive hills

            // Detail layer: audio-reactive chop (2m tall, fine detail)
            detail_amplitude_m: 2.0,
            detail_frequency: 0.1,

            base_line_width: 0.02,
            noise_seed: 42,
        }
    }
}

/// Mapping from audio frequency bands to visual parameters
#[derive(Debug, Clone)]
pub struct AudioReactiveMapping {
    /// Scale factor: bass energy → wave amplitude (meters per unit energy)
    /// toy2 value: 3.0
    /// Formula: amplitude = base_amplitude + bass * this_scale
    pub bass_to_amplitude_scale: f32,

    /// Scale factor: mid energy → wave frequency (dimensionless)
    /// toy2 value: 0.15
    /// Formula: frequency = base_frequency + mid * this_scale
    pub mid_to_frequency_scale: f32,

    /// Scale factor: high energy → line glow width
    /// toy2 value: 0.03
    /// Formula: line_width = base_line_width + high * this_scale
    pub high_to_glow_scale: f32,
}

impl Default for AudioReactiveMapping {
    fn default() -> Self {
        Self {
            bass_to_amplitude_scale: 3.0,
            mid_to_frequency_scale: 0.15,
            high_to_glow_scale: 0.03,
        }
    }
}
