//! FFT analysis thread and utilities.

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::ocean::AudioBands;
use crate::params::FFTConfig;

/// Spawn FFT analysis thread
pub fn spawn_fft_thread(
    config: FFTConfig,
    fft_buffer: Arc<Mutex<Vec<f32>>>,
    audio_bands: Arc<Mutex<AudioBands>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(config.fft_size);
        let mut fft_input = vec![Complex::new(0.0, 0.0); config.fft_size];
        let mut fft_output = vec![Complex::new(0.0, 0.0); config.fft_size];

        loop {
            thread::sleep(Duration::from_millis(config.update_interval_ms));

            let mut fft_buf = fft_buffer.lock().unwrap();

            if fft_buf.len() >= config.fft_size {
                // Apply Hann window
                for i in 0..config.fft_size {
                    let window = hann_window(i, config.fft_size);
                    fft_input[i] = Complex::new(fft_buf[i] * window, 0.0);
                }

                // Perform FFT
                fft_output.copy_from_slice(&fft_input);
                fft.process(&mut fft_output);

                // Extract frequency bands with normalization
                let bass_bins = config.bass_bins();
                let mid_bins = config.mid_bins();
                let high_bins = config.high_bins();

                let low: f32 = fft_output[bass_bins.clone()]
                    .iter()
                    .map(|c| c.norm())
                    .sum::<f32>()
                    / bass_bins.len() as f32;

                let mid: f32 = fft_output[mid_bins.clone()]
                    .iter()
                    .map(|c| c.norm())
                    .sum::<f32>()
                    / mid_bins.len() as f32;

                let high: f32 = fft_output[high_bins.clone()]
                    .iter()
                    .map(|c| c.norm())
                    .sum::<f32>()
                    / high_bins.len() as f32;

                // Update shared bands
                *audio_bands.lock().unwrap() = AudioBands { low, mid, high };

                // 50% overlap (drain half the buffer)
                fft_buf.drain(0..config.fft_size / 2);
            }
        }
    })
}

/// Hann window function for FFT analysis
pub fn hann_window(index: usize, size: usize) -> f32 {
    0.5 * (1.0 - ((2.0 * PI * index as f32) / (size as f32 - 1.0)).cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let size = 1024;

        // Hann window should be 0 at edges, 1 at center
        assert!((hann_window(0, size) - 0.0).abs() < 0.01);
        assert!((hann_window(size - 1, size) - 0.0).abs() < 0.01);
        assert!((hann_window(size / 2, size) - 1.0).abs() < 0.01);
    }
}
