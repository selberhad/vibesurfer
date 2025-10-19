//! Audio analysis configuration and constants.

use std::ops::Range;

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

/// Audio constants (compile-time, match Glicol engine setup)
pub mod audio_constants {
    /// Audio block size (samples per buffer)
    /// toy2 value: 128 (= 2.9ms @ 44.1kHz)
    pub const BLOCK_SIZE: usize = 128;
}
