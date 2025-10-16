//! Audio synthesis and FFT analysis system.
//!
//! Combines Glicol procedural synthesis with real-time FFT analysis
//! to extract frequency bands for audio-reactive visuals.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use glicol::Engine;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::ocean::AudioBands;
use crate::params::{audio_constants::BLOCK_SIZE, FFTConfig, RecordingConfig};

/// Glicol composition (procedural music code)
const GLICOL_COMPOSITION: &str = r#"
~gate: speed 2.0 >> seq 60 _60 _~a 48
~a: choose 48 48 48 72 0 0 0
~amp: ~gate >> envperc 0.001 0.1
~pit: ~gate >> mul 261.63
~lead: saw ~pit >> mul ~amp >> lpf ~mod 5.0 >> mul 0.1
~mod: sin 0.2 >> mul 1300 >> add 1500
o: ~lead >> plate 0.1
"#;

/// Audio system managing synthesis and FFT analysis
pub struct AudioSystem {
    /// Shared FFT frequency bands (thread-safe)
    audio_bands: Arc<Mutex<AudioBands>>,

    /// Audio output stream (kept alive)
    _stream: cpal::Stream,

    /// FFT analysis thread handle (optional, for cleanup)
    _fft_thread: Option<thread::JoinHandle<()>>,
}

impl AudioSystem {
    /// Create and start audio system with specified configuration
    pub fn new(
        fft_config: FFTConfig,
        recording_config: Option<RecordingConfig>,
    ) -> Result<Self, String> {
        // Validate FFT configuration
        fft_config
            .validate()
            .map_err(|e| format!("Invalid FFT config: {}", e))?;

        // Create WAV writer if recording
        let wav_writer: Option<Arc<Mutex<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>> =
            recording_config.as_ref().map(|config| {
                let spec = hound::WavSpec {
                    channels: 2,
                    sample_rate: fft_config.sample_rate_hz as u32,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                };
                let writer = hound::WavWriter::create(&config.audio_path(), spec)
                    .expect("Failed to create WAV writer");
                Arc::new(Mutex::new(writer))
            });

        let wav_writer_clone = wav_writer.clone();

        // Create Glicol engine
        let mut engine = Engine::<BLOCK_SIZE>::new();
        engine.set_sr(fft_config.sample_rate_hz);
        engine.update_with_code(GLICOL_COMPOSITION);
        engine
            .update()
            .map_err(|e| format!("Glicol engine init failed: {:?}", e))?;

        // Shared state between audio callback and FFT thread
        let engine = Arc::new(Mutex::new(engine));
        let engine_clone = Arc::clone(&engine);

        let fft_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let fft_buffer_clone = Arc::clone(&fft_buffer);

        let audio_bands = Arc::new(Mutex::new(AudioBands::default()));
        let audio_bands_fft = Arc::clone(&audio_bands);

        // Setup audio output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device found")?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get audio config: {}", e))?;

        println!(
            "Audio: {} @ {}Hz",
            device.name().unwrap_or_else(|_| "Unknown".to_string()),
            config.sample_rate().0
        );

        // Build audio output stream
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut engine = engine_clone.lock().unwrap();
                    let mut fft_buf = fft_buffer_clone.lock().unwrap();

                    let frames_needed = data.len() / 2; // Stereo frames
                    let mut frame_idx = 0;

                    // Generate multiple blocks if needed to fill the entire buffer
                    while frame_idx < frames_needed {
                        let (buffers, _) = engine.next_block(vec![]);

                        let samples_to_copy = (frames_needed - frame_idx).min(BLOCK_SIZE);

                        for i in 0..samples_to_copy {
                            // Safety limiter: hard clip to ±0.5 to prevent ear damage
                            let left = buffers[0][i].clamp(-0.5, 0.5);
                            let right = buffers[1][i].clamp(-0.5, 0.5);

                            let out_idx = (frame_idx + i) * 2;
                            data[out_idx] = left;
                            data[out_idx + 1] = right;

                            fft_buf.push(left); // Accumulate for FFT analysis

                            // Record to WAV if recording
                            if let Some(ref writer) = wav_writer_clone {
                                if let Ok(mut w) = writer.lock() {
                                    let _ = w.write_sample(left);
                                    let _ = w.write_sample(right);
                                }
                            }
                        }

                        frame_idx += samples_to_copy;
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| format!("Failed to build audio stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        // Start FFT analysis thread
        let fft_thread = spawn_fft_thread(fft_config, fft_buffer, audio_bands_fft);

        Ok(Self {
            audio_bands,
            _stream: stream,
            _fft_thread: Some(fft_thread),
        })
    }

    /// Get current audio frequency bands (thread-safe)
    pub fn get_bands(&self) -> AudioBands {
        *self.audio_bands.lock().unwrap()
    }
}

/// Spawn FFT analysis thread
fn spawn_fft_thread(
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
fn hann_window(index: usize, size: usize) -> f32 {
    0.5 * (1.0 - ((2.0 * PI * index as f32) / (size as f32 - 1.0)).cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_config_hz_to_bin() {
        let config = FFTConfig::default();

        // At 44100 Hz sample rate and 1024 FFT size:
        // Bin resolution = 44100 / 1024 ≈ 43.07 Hz per bin
        assert_eq!(config.hz_to_bin(0.0), 0);
        assert_eq!(config.hz_to_bin(43.07), 1);
        assert_eq!(config.hz_to_bin(100.0), 2); // ~100 Hz ≈ bin 2
    }

    #[test]
    fn test_fft_config_band_ranges() {
        let config = FFTConfig::default();

        let bass = config.bass_bins();
        let mid = config.mid_bins();
        let high = config.high_bins();

        // Bass: 20-200 Hz (but 20 Hz maps to bin 0, so we start at bin 0 or 1)
        assert!(bass.start >= 0); // May include DC bin at low frequencies
        assert!(bass.end <= 10);

        // Mid: 200-1000 Hz
        assert!(mid.start >= bass.end);
        assert!(mid.end <= 50);

        // High: 1000-4000 Hz
        assert!(high.start >= mid.end);
        assert!(high.end <= 200);
    }

    #[test]
    fn test_hann_window() {
        let size = 1024;

        // Hann window should be 0 at edges, 1 at center
        assert!((hann_window(0, size) - 0.0).abs() < 0.01);
        assert!((hann_window(size - 1, size) - 0.0).abs() < 0.01);
        assert!((hann_window(size / 2, size) - 1.0).abs() < 0.01);
    }
}
