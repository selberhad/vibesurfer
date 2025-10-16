//! High-level ocean system with audio-reactive modulation.

use glam::Vec3;

use super::mesh::OceanGrid;
use super::AudioBands;
use crate::params::{AudioReactiveMapping, OceanPhysics};

/// High-level ocean system with physics and audio-reactive parameters
pub struct OceanSystem {
    pub grid: OceanGrid,
    pub physics: OceanPhysics,
    mapping: AudioReactiveMapping,
}

impl OceanSystem {
    /// Create new ocean system with specified parameters
    pub fn new(physics: OceanPhysics, mapping: AudioReactiveMapping) -> Self {
        let grid = OceanGrid::new(&physics);
        Self {
            grid,
            physics,
            mapping,
        }
    }

    /// Update ocean simulation with audio-reactive modulation
    ///
    /// Audio modulation only affects detail layer (ripples), not base terrain (hills).
    /// This preserves stable skiing physics while adding visual reactivity.
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `audio_bands` - FFT frequency band energies
    /// * `camera_pos` - Camera position for infinite ocean
    ///
    /// # Returns
    /// * Tuple of (detail_amplitude, detail_frequency, line_width) for rendering
    pub fn update(
        &mut self,
        time_s: f32,
        audio_bands: &AudioBands,
        camera_pos: Vec3,
    ) -> (f32, f32, f32) {
        // Map audio bands to detail layer parameters (not base terrain)
        let detail_amplitude = self.physics.detail_amplitude_m
            + audio_bands.low * self.mapping.bass_to_amplitude_scale;

        let detail_frequency =
            self.physics.detail_frequency + audio_bands.mid * self.mapping.mid_to_frequency_scale;

        let line_width =
            self.physics.base_line_width + audio_bands.high * self.mapping.high_to_glow_scale;

        // Update mesh vertices (base terrain + audio-reactive detail)
        self.grid.update(
            time_s,
            detail_amplitude,
            detail_frequency,
            camera_pos,
            &self.physics,
        );

        (detail_amplitude, detail_frequency, line_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_reactive_mapping() {
        let physics = OceanPhysics::default();
        let mapping = AudioReactiveMapping::default();
        let mut ocean = OceanSystem::new(physics, mapping);

        let bands = AudioBands {
            low: 1.0,
            mid: 0.5,
            high: 0.2,
        };

        let (amplitude, frequency, line_width) = ocean.update(0.0, &bands, Vec3::ZERO);

        // Check that audio modulation is applied
        assert!(amplitude > ocean.physics.detail_amplitude_m);
        assert!(frequency > ocean.physics.detail_frequency);
        assert!(line_width > ocean.physics.base_line_width);
    }
}
