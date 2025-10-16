//! Procedural camera journey system with parameterized cinematic paths.

use glam::{Mat4, Vec3};

use crate::params::{CameraJourney, RenderConfig};

/// Camera system with procedural journey path
pub struct CameraSystem {
    params: CameraJourney,
}

impl CameraSystem {
    /// Create new camera system with specified journey parameters
    pub fn new(params: CameraJourney) -> Self {
        Self { params }
    }

    /// Compute camera position and look-at target for given time
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    ///
    /// # Returns
    /// Tuple of (eye_position, target_position)
    pub fn compute_position_and_target(&self, time_s: f32) -> (Vec3, Vec3) {
        let p = &self.params;

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

    /// Create view-projection matrix for rendering
    ///
    /// # Arguments
    /// * `time_s` - Current time in seconds
    /// * `render_config` - Rendering configuration (FOV, aspect ratio, etc.)
    ///
    /// # Returns
    /// Tuple of (view_proj_matrix, camera_position)
    pub fn create_view_proj_matrix(
        &self,
        time_s: f32,
        render_config: &RenderConfig,
    ) -> (Mat4, Vec3) {
        let (eye, target) = self.compute_position_and_target(time_s);

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
    fn test_camera_position_at_t0() {
        let camera = CameraSystem::new(CameraJourney::default());
        let (eye, target) = camera.compute_position_and_target(0.0);

        // At t=0, most sine waves are at 0, cosines at 1
        // Y should be clamped to at least y_min_altitude_m
        assert!(eye.y >= camera.params.y_min_altitude_m);

        // Target should be ahead of camera in Z
        assert!(target.z > eye.z);
    }

    #[test]
    fn test_camera_altitude_clamping() {
        let mut params = CameraJourney::default();
        params.y_base_altitude_m = 10.0;
        params.y_swoop_amplitude_m = 50.0; // Would go negative without clamping
        params.y_min_altitude_m = 20.0;

        let camera = CameraSystem::new(params);

        // Test various time points
        for t in 0..100 {
            let (eye, _) = camera.compute_position_and_target(t as f32 * 0.1);
            assert!(
                eye.y >= camera.params.y_min_altitude_m,
                "Altitude {} below minimum {} at t={}",
                eye.y,
                camera.params.y_min_altitude_m,
                t
            );
        }
    }

    #[test]
    fn test_view_proj_matrix_generation() {
        let camera = CameraSystem::new(CameraJourney::default());
        let render_config = RenderConfig::default();

        let (view_proj, eye_pos) = camera.create_view_proj_matrix(0.0, &render_config);

        // Matrix should not be identity or zero
        assert_ne!(view_proj, Mat4::IDENTITY);
        assert_ne!(view_proj, Mat4::ZERO);

        // Eye position should be valid (not NaN or infinite)
        assert!(eye_pos.x.is_finite());
        assert!(eye_pos.y.is_finite());
        assert!(eye_pos.z.is_finite());
    }
}
