//! Parameter definitions with physical units and documented semantics.
//!
//! All magic numbers are extracted here with:
//! - Physical units (meters, seconds, Hz, etc.)
//! - Documented ranges and meanings
//! - Type safety where possible

mod audio;
mod camera;
mod ocean;
mod render;

// Re-export all types
pub use audio::{audio_constants, FFTConfig};
pub use camera::{BasicCameraPath, CameraJourney, CameraPreset, FixedCamera, FloatingCamera};
pub use ocean::{AudioReactiveMapping, OceanPhysics};
pub use render::{RecordingConfig, RenderConfig};
