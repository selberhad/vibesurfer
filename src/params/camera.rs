//! Camera path configuration and presets.

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

/// Fixed camera position (for debugging)
#[derive(Debug, Clone)]
pub struct FixedCamera {
    /// Camera position (meters)
    pub position: [f32; 3],

    /// Look-at target (meters)
    pub target: [f32; 3],

    /// Simulated forward velocity (m/s) to flow the grid
    pub simulated_velocity: f32,
}

impl Default for FixedCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 101.0, 0.0], // Just above tallest hills (100m amplitude)
            target: [0.0, 0.0, 100.0],   // Looking forward and down
            simulated_velocity: 150.0,   // Same as basic preset
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

    /// Fixed preset: stationary camera for debugging
    Fixed(FixedCamera),
}

impl Default for CameraPreset {
    fn default() -> Self {
        Self::Fixed(FixedCamera::default())
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
