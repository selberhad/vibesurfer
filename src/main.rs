//! Vibesurfer - A fluid, retro-futuristic jet-surfing simulator
//!
//! The surface behaves like living music: waves pulse to the beat,
//! currents shimmer with color, and your motion becomes rhythm.

use clap::Parser;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use glam::Mat4;
use vibesurfer::audio::AudioSystem;
use vibesurfer::camera::CameraSystem;
use vibesurfer::cli::Args;
use vibesurfer::ocean::OceanSystem;
use vibesurfer::params::*;
use vibesurfer::rendering::{RenderSystem, SkyboxUniforms, Uniforms};

/// Main application state
struct App {
    // Window and rendering
    window: Option<Arc<Window>>,
    render_system: Option<RenderSystem>,

    // Simulation systems
    ocean: OceanSystem,
    camera: CameraSystem,
    audio: Option<AudioSystem>,

    // Configuration
    render_config: RenderConfig,
    recording_config: Option<RecordingConfig>,

    // Time tracking
    start_time: Instant,
    frame_count: usize,
    last_fps_update: Instant,
    last_fps_frame_count: usize,
    fps: f32,
}

impl App {
    fn new(camera_preset: CameraPreset, recording_config: Option<RecordingConfig>) -> Self {
        // Create default parameters
        let ocean_physics = OceanPhysics::default();
        let audio_mapping = AudioReactiveMapping::default();
        let render_config = RenderConfig::default();

        // Initialize systems
        let ocean = OceanSystem::new(ocean_physics, audio_mapping);
        let camera = CameraSystem::new(camera_preset);

        let now = Instant::now();
        Self {
            window: None,
            render_system: None,
            ocean,
            camera,
            audio: None,
            render_config,
            recording_config,
            start_time: now,
            frame_count: 0,
            last_fps_update: now,
            last_fps_frame_count: 0,
            fps: 0.0,
        }
    }

    fn is_recording(&self) -> bool {
        self.recording_config.is_some()
    }
}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already initialized
        }

        // Create window
        let window_attributes = Window::default_attributes()
            .with_title("Vibesurfer - Audio-Reactive Ocean")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.render_config.window_width,
                self.render_config.window_height,
            ));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Initialize rendering system
        let render_system = pollster::block_on(RenderSystem::new(
            Arc::clone(&window),
            &self.ocean.grid,
            self.recording_config.clone(),
        ))
        .unwrap();

        // Initialize audio system
        let fft_config = FFTConfig::default();
        let audio = AudioSystem::new(fft_config, self.recording_config.clone()).unwrap();

        if self.is_recording() {
            let cfg = self.recording_config.as_ref().unwrap();
            println!("\n🎬 Recording mode: {} seconds", cfg.duration_secs);
            println!("   Output: {}/", cfg.output_dir);
            println!("   Frames: {} @ {}fps", cfg.total_frames(), cfg.fps);
        } else {
            println!("\nVibesurfer is running!");
            println!("Press ESC to quit\n");
        }

        self.window = Some(window);
        self.render_system = Some(render_system);
        self.audio = Some(audio);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.render_frame();

                // Check if recording is complete
                if self.is_recording() {
                    let cfg = self.recording_config.as_ref().unwrap();
                    if self.frame_count >= cfg.total_frames() {
                        println!(
                            "\n✅ Recording complete! {} frames captured",
                            self.frame_count
                        );
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}

impl App {
    /// Render a single frame
    fn render_frame(&mut self) {
        let Some(ref render_system) = self.render_system else {
            return;
        };
        let Some(ref audio) = self.audio else {
            return;
        };

        // Get current time
        let time_s = self.start_time.elapsed().as_secs_f32();

        // Get audio frequency bands
        let audio_bands = audio.get_bands();

        // Create terrain query function for floating camera
        let ocean_physics = self.ocean.physics.clone();
        let terrain_fn = |x: f32, z: f32| self.ocean.grid.query_base_terrain(x, z, &ocean_physics);

        // Update camera position
        let (view_proj, camera_pos) =
            self.camera
                .create_view_proj_matrix(time_s, &self.render_config, Some(terrain_fn));

        // === Terrain Generation: GPU only ===

        let (amplitude, frequency, line_width, index_count) = {
            // GPU path: Compute audio-modulated parameters
            let amplitude = self.ocean.physics.detail_amplitude_m
                + audio_bands.low * self.ocean.mapping.bass_to_amplitude_scale;
            let frequency = self.ocean.physics.detail_frequency
                + audio_bands.mid * self.ocean.mapping.mid_to_frequency_scale;
            let line_width = self.ocean.physics.base_line_width
                + audio_bands.high * self.ocean.mapping.high_to_glow_scale;

            // Create terrain params for GPU (camera at actual world position)
            let terrain_params = vibesurfer::params::TerrainParams {
                base_amplitude: self.ocean.physics.base_terrain_amplitude_m,
                base_frequency: self.ocean.physics.base_terrain_frequency,
                detail_amplitude: amplitude,
                detail_frequency: frequency,
                camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z],
                _padding1: 0.0,
                grid_size: self.ocean.physics.grid_size as u32,
                grid_spacing: self.ocean.physics.grid_spacing_m,
                time: time_s * self.ocean.physics.wave_speed,
                _padding2: 0.0,
            };

            // Dispatch GPU compute shader
            render_system
                .dispatch_terrain_compute(&terrain_params, self.ocean.physics.grid_size as u32);

            // Use all indices (no phantom line filtering in Phase 1)
            let index_count = self.ocean.grid.indices.len() as u32;

            (amplitude, frequency, line_width, index_count)
        };

        // Grid is local window around camera (camera moves through world space)
        let model = Mat4::IDENTITY;
        let mvp = view_proj * model;

        // Update ocean uniforms
        let uniforms = Uniforms {
            view_proj: mvp.to_cols_array_2d(),
            line_width,
            amplitude,
            frequency,
            time: time_s,
        };
        render_system.update_uniforms(&uniforms);

        // Update skybox uniforms
        let inv_view_proj = view_proj.inverse();
        let skybox_uniforms = SkyboxUniforms {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            time: time_s,
            _padding: [0.0; 3],
        };
        render_system.update_skybox_uniforms(&skybox_uniforms);

        // Render (and capture if recording)
        if let Err(e) = render_system.render(self.frame_count, index_count) {
            eprintln!("Render error: {:?}", e);
        }

        self.frame_count += 1;

        // Update FPS in window title every 0.5 seconds
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();
        if elapsed >= 0.5 {
            let frames_elapsed = self.frame_count - self.last_fps_frame_count;
            self.fps = frames_elapsed as f32 / elapsed;
            self.last_fps_update = now;
            self.last_fps_frame_count = self.frame_count;

            if let Some(ref window) = self.window {
                // TODO(Phase B): Add velocity display back using camera position delta
                window.set_title(&format!(
                    "Vibesurfer - Audio-Reactive Ocean | {:.0} FPS",
                    self.fps
                ));
            }
        }
    }
}

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    println!("Vibesurfer - Fluid audio-reactive ocean surfing simulator");
    println!("Initializing systems...\n");

    // Parse camera preset and recording config
    let camera_preset = args.parse_camera_preset();
    let recording_config = args.create_recording_config();

    let mut app = App::new(camera_preset, recording_config);
    let event_loop = EventLoop::new().unwrap();
    let _ = event_loop.run_app(&mut app);
}
