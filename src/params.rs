//! Parameter definitions with physical units and documented semantics.
//!
//! All magic numbers from toy2 are extracted here with:
//! - Physical units (meters, seconds, Hz, etc.)
//! - Documented ranges and meanings
//! - Type safety where possible

use std::ops::Range;

/// Ocean simulation physics parameters
#[derive(Debug, Clone)]
pub struct OceanPhysics {
    /// Grid resolution (vertices per side, e.g., 128 = 16,641 vertices)
    pub grid_size: usize,

    /// Spacing between grid vertices in world units (meters)
    /// toy2 value: 10.0
    pub grid_spacing_m: f32,

    /// Wave animation speed multiplier (dimensionless, affects time scaling)
    /// toy2 value: 0.5
    pub wave_speed: f32,

    /// Base wave height in meters (before audio modulation)
    /// toy2 value: 2.0
    pub base_amplitude_m: f32,

    /// Base spatial frequency (cycles per meter, controls wave detail)
    /// toy2 value: 0.1
    pub base_frequency: f32,

    /// Base wireframe line width (screen-space or shader units)
    /// toy2 value: 0.02
    pub base_line_width: f32,

    /// Perlin noise seed
    /// toy2 value: 42
    pub noise_seed: u32,
}

impl Default for OceanPhysics {
    fn default() -> Self {
        Self {
            grid_size: 512,      // Large enough for good view distance without lag
            grid_spacing_m: 2.0, // Fine spacing for many lines
            wave_speed: 0.5,
            base_amplitude_m: 2.0,
            base_frequency: 0.1,
            base_line_width: 0.02,
            noise_seed: 42,
        }
    }
}

// Helper methods removed (were unused)

/// FFT analysis configuration with frequency band mappings
#[derive(Debug, Clone)]
pub struct FFTConfig {
    /// Audio sample rate (Hz)
    /// toy2 value: 44100
    pub sample_rate_hz: usize,

    /// FFT window size (must be power of 2)
    /// toy2 value: 1024
    pub fft_size: usize,

    /// FFT update interval (milliseconds)
    /// toy2 value: 50 (= 20 Hz update rate)
    pub update_interval_ms: u64,

    /// Bass frequency range (Hz)
    /// toy2 bins: 1..10 ≈ 20-200 Hz
    pub bass_range_hz: (f32, f32),

    /// Mid frequency range (Hz)
    /// toy2 bins: 10..50 ≈ 200-1000 Hz
    pub mid_range_hz: (f32, f32),

    /// High frequency range (Hz)
    /// toy2 bins: 50..200 ≈ 1000-4000 Hz
    pub high_range_hz: (f32, f32),
}

impl Default for FFTConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 44100,
            fft_size: 1024,
            update_interval_ms: 50,
            bass_range_hz: (20.0, 200.0),
            mid_range_hz: (200.0, 1000.0),
            high_range_hz: (1000.0, 4000.0),
        }
    }
}

impl FFTConfig {
    /// Convert frequency (Hz) to FFT bin index
    pub fn hz_to_bin(&self, hz: f32) -> usize {
        ((hz * self.fft_size as f32) / self.sample_rate_hz as f32) as usize
    }

    /// Get FFT bin range for bass frequencies
    pub fn bass_bins(&self) -> Range<usize> {
        self.hz_to_bin(self.bass_range_hz.0)..self.hz_to_bin(self.bass_range_hz.1)
    }

    /// Get FFT bin range for mid frequencies
    pub fn mid_bins(&self) -> Range<usize> {
        self.hz_to_bin(self.mid_range_hz.0)..self.hz_to_bin(self.mid_range_hz.1)
    }

    /// Get FFT bin range for high frequencies
    pub fn high_bins(&self) -> Range<usize> {
        self.hz_to_bin(self.high_range_hz.0)..self.hz_to_bin(self.high_range_hz.1)
    }

    /// Validate configuration (FFT size must be power of 2, etc.)
    pub fn validate(&self) -> Result<(), String> {
        if !self.fft_size.is_power_of_two() {
            return Err(format!(
                "FFT size must be power of 2, got {}",
                self.fft_size
            ));
        }
        if self.sample_rate_hz == 0 {
            return Err("Sample rate must be > 0".to_string());
        }
        Ok(())
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

/// Basic camera path parameters (simple straight-line flight)
#[derive(Debug, Clone)]
pub struct BasicCameraPath {
    /// Constant altitude (meters)
    pub altitude_m: f32,

    /// Forward movement speed (meters per second)
    pub forward_speed_m_per_s: f32,

    /// Look-ahead distance (meters)
    pub look_ahead_m: f32,
}

impl Default for BasicCameraPath {
    fn default() -> Self {
        Self {
            altitude_m: 30.0,             // Moderate altitude
            forward_speed_m_per_s: 150.0, // Fast speed
            look_ahead_m: 150.0,
        }
    }
}

/// Camera preset selection
#[derive(Debug, Clone)]
pub enum CameraPreset {
    /// Cinematic preset: complex procedural path with sweeping arcs and altitude changes
    Cinematic(CameraJourney),

    /// Basic preset: straight-line flight at constant altitude, looking forward
    Basic(BasicCameraPath),
}

impl Default for CameraPreset {
    fn default() -> Self {
        Self::Basic(BasicCameraPath::default())
    }
}

/// Camera journey path parameters (procedural cinematic path)
#[derive(Debug, Clone)]
pub struct CameraJourney {
    // X axis: Wide sweeping arcs
    /// Primary X oscillation frequency (Hz)
    /// toy2 value: 0.2
    pub x_freq_primary_hz: f32,

    /// Primary X oscillation amplitude (meters)
    /// toy2 value: 80.0
    pub x_amplitude_primary_m: f32,

    /// Secondary X oscillation frequency (Hz)
    /// toy2 value: 0.7
    pub x_freq_secondary_hz: f32,

    /// Secondary X oscillation amplitude (meters)
    /// toy2 value: 30.0
    pub x_amplitude_secondary_m: f32,

    // Z axis: Forward progression + weaving
    /// Forward movement speed (meters per second)
    /// toy2 value: 8.0
    pub z_forward_speed_m_per_s: f32,

    /// Primary Z weave frequency (Hz)
    /// toy2 value: 0.5
    pub z_weave_freq_primary_hz: f32,

    /// Primary Z weave amplitude (meters)
    /// toy2 value: 40.0
    pub z_weave_amplitude_primary_m: f32,

    /// Secondary Z weave frequency (Hz)
    /// toy2 value: 1.1
    pub z_weave_freq_secondary_hz: f32,

    /// Secondary Z weave amplitude (meters)
    /// toy2 value: 20.0
    pub z_weave_amplitude_secondary_m: f32,

    // Y axis: Altitude with swoops
    /// Base altitude (meters)
    /// toy2 value: 80.0
    pub y_base_altitude_m: f32,

    /// Primary Y swoop frequency (Hz)
    /// toy2 value: 0.3
    pub y_swoop_freq_hz: f32,

    /// Primary Y swoop amplitude (meters, ±)
    /// toy2 value: 30.0
    pub y_swoop_amplitude_m: f32,

    /// Secondary Y detail frequency (Hz)
    /// toy2 value: 1.3
    pub y_detail_freq_hz: f32,

    /// Secondary Y detail amplitude (meters, ±)
    /// toy2 value: 10.0
    pub y_detail_amplitude_m: f32,

    /// Minimum altitude clamp (meters, prevents camera going too low)
    /// toy2 value: 50.0
    pub y_min_altitude_m: f32,

    // Look-at target
    /// Look-at X pan frequency (Hz)
    /// toy2 value: 0.4
    pub target_x_pan_freq_hz: f32,

    /// Look-at X pan amplitude (meters)
    /// toy2 value: 50.0
    pub target_x_pan_amplitude_m: f32,

    /// Look-at Z distance ahead (meters)
    /// toy2 value: 200.0
    pub target_z_ahead_m: f32,

    /// Look-at Z oscillation frequency (Hz)
    /// toy2 value: 0.6
    pub target_z_osc_freq_hz: f32,

    /// Look-at Z oscillation amplitude (meters)
    /// toy2 value: 30.0
    pub target_z_osc_amplitude_m: f32,

    /// Look-at Y multiplier (fraction of camera altitude)
    /// toy2 value: 0.7
    pub target_y_altitude_fraction: f32,

    /// Look-at Y oscillation frequency (Hz)
    /// toy2 value: 0.5
    pub target_y_osc_freq_hz: f32,

    /// Look-at Y oscillation amplitude (meters)
    /// toy2 value: 20.0
    pub target_y_osc_amplitude_m: f32,
}

impl Default for CameraJourney {
    fn default() -> Self {
        Self {
            // X axis sweeping arcs
            x_freq_primary_hz: 0.2,
            x_amplitude_primary_m: 80.0,
            x_freq_secondary_hz: 0.7,
            x_amplitude_secondary_m: 30.0,

            // Z axis forward + weaving
            z_forward_speed_m_per_s: 8.0,
            z_weave_freq_primary_hz: 0.5,
            z_weave_amplitude_primary_m: 40.0,
            z_weave_freq_secondary_hz: 1.1,
            z_weave_amplitude_secondary_m: 20.0,

            // Y axis altitude
            y_base_altitude_m: 80.0,
            y_swoop_freq_hz: 0.3,
            y_swoop_amplitude_m: 30.0,
            y_detail_freq_hz: 1.3,
            y_detail_amplitude_m: 10.0,
            y_min_altitude_m: 50.0,

            // Look-at target
            target_x_pan_freq_hz: 0.4,
            target_x_pan_amplitude_m: 50.0,
            target_z_ahead_m: 200.0,
            target_z_osc_freq_hz: 0.6,
            target_z_osc_amplitude_m: 30.0,
            target_y_altitude_fraction: 0.7,
            target_y_osc_freq_hz: 0.5,
            target_y_osc_amplitude_m: 20.0,
        }
    }
}

/// Rendering configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Window width (pixels)
    pub window_width: u32,

    /// Window height (pixels)
    pub window_height: u32,

    /// Field of view (degrees)
    /// 75° = wide perspective for sense of speed and vastness
    pub fov_degrees: f32,

    /// Near clipping plane (meters)
    /// toy2 value: 0.1
    pub near_plane_m: f32,

    /// Far clipping plane (meters)
    /// Extended to 2000m for more visible ocean horizon
    pub far_plane_m: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            window_width: 1280,
            window_height: 720,
            fov_degrees: 100.0, // Very wide FOV for extreme perspective
            near_plane_m: 0.1,
            far_plane_m: 2000.0,
        }
    }
}

impl RenderConfig {
    pub fn aspect_ratio(&self) -> f32 {
        self.window_width as f32 / self.window_height as f32
    }
}

/// Audio constants (compile-time, match Glicol engine setup)
pub mod audio_constants {
    /// Audio block size (samples per buffer)
    /// toy2 value: 128 (= 2.9ms @ 44.1kHz)
    pub const BLOCK_SIZE: usize = 128;
}

/// Recording mode configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Duration to record (seconds)
    pub duration_secs: f32,

    /// Output directory for frames and audio
    pub output_dir: String,

    /// Frame rate (FPS)
    pub fps: u32,
}

impl RecordingConfig {
    pub fn new(duration_secs: f32) -> Self {
        Self {
            duration_secs,
            output_dir: "recording".to_string(),
            fps: 60,
        }
    }

    /// Total number of frames to capture
    pub fn total_frames(&self) -> usize {
        (self.duration_secs * self.fps as f32).ceil() as usize
    }

    /// Frame directory path
    pub fn frames_dir(&self) -> String {
        format!("{}/frames", self.output_dir)
    }

    /// Audio file path
    pub fn audio_path(&self) -> String {
        format!("{}/audio.wav", self.output_dir)
    }
}
