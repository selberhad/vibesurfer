//! Procedural camera journey system with parameterized cinematic paths.

use glam::{Mat4, Vec3};

use crate::params::{
    BasicCameraPath, CameraJourney, CameraPreset, FixedCamera, FloatingCamera, RenderConfig,
};

/// Type alias for terrain height query function (saves boilerplate in tests)
type TerrainFn = fn(f32, f32) -> f32;

/// Camera system with procedural journey path
pub struct CameraSystem {
    preset: CameraPreset,
}

impl CameraSystem {
    /// Create new camera system with specified preset
    pub fn new(preset: CameraPreset) -> Self {
        Self { preset }
    }

    /// Compute camera position and look-at target for given time
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `terrain_height_fn` - Optional function to query terrain height at (x, z) world position
    ///
    /// # Returns
    /// Tuple of (eye_position, target_position)
    pub fn compute_position_and_target<F>(
        &self,
        time_s: f32,
        terrain_height_fn: Option<F>,
    ) -> (Vec3, Vec3)
    where
        F: Fn(f32, f32) -> f32,
    {
        match &self.preset {
            CameraPreset::Cinematic(params) => Self::compute_cinematic_path(params, time_s),
            CameraPreset::Basic(params) => Self::compute_basic_path(params, time_s),
            CameraPreset::Fixed(params) => Self::compute_fixed_path(params, time_s),
            CameraPreset::Floating(params) => {
                if let Some(ref get_height) = terrain_height_fn {
                    Self::compute_floating_path(params, time_s, get_height)
                } else {
                    // Fallback if no terrain query available
                    Self::compute_fixed_path(&FixedCamera::default(), time_s)
                }
            }
        }
    }

    /// Compute fixed camera path (moves forward at constant velocity)
    fn compute_fixed_path(p: &FixedCamera, time_s: f32) -> (Vec3, Vec3) {
        // Camera moves forward through world space
        let eye = Vec3::new(
            p.position[0],                 // X: stays constant
            p.position[1],                 // Y: stays at elevation
            time_s * p.simulated_velocity, // Z: moves forward
        );

        // Target moves with camera, maintaining relative offset
        let target_offset = Vec3::from_array(p.target) - Vec3::from_array(p.position);
        let target = eye + target_offset;

        (eye, target)
    }

    /// Compute floating camera path (follows terrain contour, actually moves through world)
    fn compute_floating_path<F>(p: &FloatingCamera, time_s: f32, get_height: F) -> (Vec3, Vec3)
    where
        F: Fn(f32, f32) -> f32,
    {
        // Calculate distance traveled with acceleration: s = v0*t + 0.5*a*t²
        let distance = p.initial_velocity * time_s + 0.5 * p.acceleration * time_s * time_s;

        // Camera position in world space (actually moves forward)
        let x = p.position_xz[0];
        let z = p.position_xz[1] + distance;

        // Query terrain at camera's actual position
        let terrain_height = get_height(x, z);
        let y = terrain_height + p.height_above_terrain_m;

        let eye = Vec3::new(x, y, z);

        // Look-at target (also in world space, ahead of camera)
        let target_x = x;
        let target_z = z + p.look_ahead_m;
        let target_terrain_height = get_height(target_x, target_z);
        let target_y = target_terrain_height + p.height_above_terrain_m * 0.6; // Look slightly down

        let target = Vec3::new(target_x, target_y, target_z);

        (eye, target)
    }

    /// Compute cinematic camera path (complex procedural motion)
    fn compute_cinematic_path(p: &CameraJourney, time_s: f32) -> (Vec3, Vec3) {
        // X axis: Wide sweeping arcs using layered sine waves
        let x = (time_s * p.x_freq_primary_hz).sin() * p.x_amplitude_primary_m
            + (time_s * p.x_freq_secondary_hz).cos() * p.x_amplitude_secondary_m;

        // Z axis: Forward progression with side-to-side weaving
        let z_forward = time_s * p.z_forward_speed_m_per_s;
        let z_weave = (time_s * p.z_weave_freq_primary_hz).sin() * p.z_weave_amplitude_primary_m
            + (time_s * p.z_weave_freq_secondary_hz).cos() * p.z_weave_amplitude_secondary_m;
        let z = z_forward + z_weave;

        // Y axis: Base altitude with swooping climbs and dives
        let y_swoop = (time_s * p.y_swoop_freq_hz).sin() * p.y_swoop_amplitude_m;
        let y_detail = (time_s * p.y_detail_freq_hz).sin() * p.y_detail_amplitude_m;
        let y = (p.y_base_altitude_m + y_swoop + y_detail).max(p.y_min_altitude_m);

        let eye = Vec3::new(x, y, z);

        // Look-at target: Looks toward horizon, slightly ahead and panning
        let target_x = x + (time_s * p.target_x_pan_freq_hz).sin() * p.target_x_pan_amplitude_m;
        let target_z = z
            + p.target_z_ahead_m
            + (time_s * p.target_z_osc_freq_hz).cos() * p.target_z_osc_amplitude_m;
        let target_y = y * p.target_y_altitude_fraction
            + (time_s * p.target_y_osc_freq_hz).sin() * p.target_y_osc_amplitude_m;
        let target = Vec3::new(target_x, target_y, target_z);

        (eye, target)
    }

    /// Compute basic camera path (straight line, constant altitude)
    fn compute_basic_path(p: &BasicCameraPath, time_s: f32) -> (Vec3, Vec3) {
        // Simple straight-line motion
        let x = 0.0; // Stay centered
        let y = p.altitude_m; // Constant altitude
        let z = time_s * p.forward_speed_m_per_s; // Linear forward motion

        let eye = Vec3::new(x, y, z);

        // Look slightly down toward the ocean surface to see motion
        // Target is ahead and below eye level (creates ~15-20 degree downward angle)
        let target_y = y * 0.6; // Look at point 40% lower than camera
        let target = Vec3::new(x, target_y, z + p.look_ahead_m);

        (eye, target)
    }

    /// Create view-projection matrix for rendering
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `render_config` - Rendering configuration (FOV, aspect ratio, etc.)
    /// * `terrain_height_fn` - Optional function to query terrain height (required for Floating preset)
    ///
    /// # Returns
    /// Tuple of (view_proj_matrix, camera_position)
    pub fn create_view_proj_matrix<F>(
        &self,
        time_s: f32,
        render_config: &RenderConfig,
        terrain_height_fn: Option<F>,
    ) -> (Mat4, Vec3)
    where
        F: Fn(f32, f32) -> f32,
    {
        let (eye, target) = self.compute_position_and_target(time_s, terrain_height_fn);

        // Always keep Y as up vector (camera never rolls)
        let up = Vec3::Y;

        let view = Mat4::look_at_rh(eye, target, up);
        let proj = Mat4::perspective_rh(
            render_config.fov_degrees.to_radians(),
            render_config.aspect_ratio(),
            render_config.near_plane_m,
            render_config.far_plane_m,
        );

        (proj * view, eye)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cinematic_camera_position_at_t0() {
        let preset = CameraPreset::Cinematic(CameraJourney::default());
        let camera = CameraSystem::new(preset.clone());
        let (eye, target) = camera.compute_position_and_target(0.0, None::<TerrainFn>);

        // At t=0, most sine waves are at 0, cosines at 1
        // Y should be clamped to at least y_min_altitude_m
        if let CameraPreset::Cinematic(params) = &preset {
            assert!(eye.y >= params.y_min_altitude_m);
        }

        // Target should be ahead of camera in Z
        assert!(target.z > eye.z);
    }

    #[test]
    fn test_cinematic_camera_altitude_clamping() {
        let mut params = CameraJourney::default();
        params.y_base_altitude_m = 10.0;
        params.y_swoop_amplitude_m = 50.0; // Would go negative without clamping
        params.y_min_altitude_m = 20.0;

        let camera = CameraSystem::new(CameraPreset::Cinematic(params.clone()));

        // Test various time points
        for t in 0..100 {
            let (eye, _) = camera.compute_position_and_target(t as f32 * 0.1, None::<TerrainFn>);
            assert!(
                eye.y >= params.y_min_altitude_m,
                "Altitude {} below minimum {} at t={}",
                eye.y,
                params.y_min_altitude_m,
                t
            );
        }
    }

    #[test]
    fn test_basic_camera_straight_line() {
        let params = BasicCameraPath::default();
        let camera = CameraSystem::new(CameraPreset::Basic(params.clone()));

        // Test at t=0
        let (eye0, target0) = camera.compute_position_and_target(0.0, None::<TerrainFn>);
        assert_eq!(eye0.x, 0.0); // Centered
        assert_eq!(eye0.y, params.altitude_m); // Constant altitude
        assert_eq!(eye0.z, 0.0); // Starting position

        // Test at t=1
        let (eye1, target1) = camera.compute_position_and_target(1.0, None::<TerrainFn>);
        assert_eq!(eye1.x, 0.0); // Still centered
        assert_eq!(eye1.y, params.altitude_m); // Still same altitude
        assert_eq!(eye1.z, params.forward_speed_m_per_s); // Moved forward

        // Target always ahead by look_ahead_m
        assert_eq!(target0.z, eye0.z + params.look_ahead_m);
        assert_eq!(target1.z, eye1.z + params.look_ahead_m);
    }

    #[test]
    fn test_view_proj_matrix_generation() {
        let camera = CameraSystem::new(CameraPreset::default());
        let render_config = RenderConfig::default();

        let (view_proj, eye_pos) =
            camera.create_view_proj_matrix(0.0, &render_config, None::<TerrainFn>);

        // Matrix should not be identity or zero
        assert_ne!(view_proj, Mat4::IDENTITY);
        assert_ne!(view_proj, Mat4::ZERO);

        // Eye position should be valid (not NaN or infinite)
        assert!(eye_pos.x.is_finite());
        assert!(eye_pos.y.is_finite());
        assert!(eye_pos.z.is_finite());
    }
}
