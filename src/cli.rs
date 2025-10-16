//! Command-line argument parsing.

use clap::Parser;

use crate::params::{
    BasicCameraPath, CameraJourney, CameraPreset, FixedCamera, FloatingCamera, RecordingConfig,
};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "Vibesurfer")]
#[command(about = "Audio-reactive ocean surfing simulator", long_about = None)]
pub struct Args {
    /// Record gameplay to video (duration in seconds)
    #[arg(long, value_name = "SECONDS")]
    pub record: Option<f32>,

    /// Camera preset: fixed (default), basic, cinematic, floating
    #[arg(long, value_name = "PRESET", default_value = "fixed")]
    pub camera_preset: String,

    /// Camera elevation for fixed preset (meters above origin)
    #[arg(long, value_name = "METERS", default_value = "101")]
    pub elevation: f32,

    /// Height above terrain for floating preset (meters)
    #[arg(long, value_name = "METERS", default_value = "20")]
    pub float_height: f32,
}

impl Args {
    /// Parse camera preset from command-line arguments
    pub fn parse_camera_preset(&self) -> CameraPreset {
        match self.camera_preset.to_lowercase().as_str() {
            "basic" => {
                println!("Camera: Basic (straight-line flight)");
                CameraPreset::Basic(BasicCameraPath::default())
            }
            "cinematic" => {
                println!("Camera: Cinematic (procedural journey)");
                CameraPreset::Cinematic(CameraJourney::default())
            }
            "fixed" => {
                println!("Camera: Fixed (elevation: {}m)", self.elevation);
                let mut fixed = FixedCamera::default();
                fixed.position[1] = self.elevation;
                CameraPreset::Fixed(fixed)
            }
            "floating" => {
                println!("Camera: Floating ({}m above terrain)", self.float_height);
                let mut floating = FloatingCamera::default();
                floating.height_above_terrain_m = self.float_height;
                CameraPreset::Floating(floating)
            }
            other => {
                eprintln!("Warning: Unknown camera preset '{}', using fixed", other);
                CameraPreset::Fixed(FixedCamera::default())
            }
        }
    }

    /// Create recording configuration if recording mode is enabled
    pub fn create_recording_config(&self) -> Option<RecordingConfig> {
        self.record.map(|duration| {
            let config = RecordingConfig::new(duration);

            // Create output directories
            std::fs::create_dir_all(&config.frames_dir())
                .expect("Failed to create frames directory");
            std::fs::create_dir_all(&config.output_dir).expect("Failed to create output directory");

            config
        })
    }
}
