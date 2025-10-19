//! Rendering and recording configuration.

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
            far_plane_m: 3000.0, // Enough for grid extent (2048m)
        }
    }
}

impl RenderConfig {
    pub fn aspect_ratio(&self) -> f32 {
        self.window_width as f32 / self.window_height as f32
    }
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
