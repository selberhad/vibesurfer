//! Audio synthesis and FFT analysis system.
//!
//! Combines Glicol procedural synthesis with real-time FFT analysis
//! to extract frequency bands for audio-reactive visuals.

mod fft;
mod synthesis;
mod system;

// Re-export public types
pub use system::AudioSystem;
