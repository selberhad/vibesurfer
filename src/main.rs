//! Skiwave - A fluid, retro-futuristic jet-surfing simulator
//!
//! The surface behaves like living music: waves pulse to the beat,
//! currents shimmer with color, and your motion becomes rhythm.

mod audio;
mod camera;
mod ocean;
mod params;
mod rendering;

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

use audio::AudioSystem;
use camera::CameraSystem;
use glam::Mat4;
use ocean::OceanSystem;
use params::*;
use rendering::{RenderSystem, SkyboxUniforms, Uniforms};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "Skiwave")]
#[command(about = "Audio-reactive ocean surfing simulator", long_about = None)]
struct Args {
    /// Record gameplay to video (duration in seconds)
    #[arg(long, value_name = "SECONDS")]
    record: Option<f32>,

    /// Camera preset: basic (default), cinematic
    #[arg(long, value_name = "PRESET", default_value = "basic")]
    camera_preset: String,
}

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
}

impl App {
    fn new(camera_preset: params::CameraPreset, recording_config: Option<RecordingConfig>) -> Self {
        // Create default parameters
        let ocean_physics = OceanPhysics::default();
        let audio_mapping = AudioReactiveMapping::default();
        let render_config = RenderConfig::default();

        // Initialize systems
        let ocean = OceanSystem::new(ocean_physics, audio_mapping);
        let camera = CameraSystem::new(camera_preset);

        Self {
            window: None,
            render_system: None,
            ocean,
            camera,
            audio: None,
            render_config,
            recording_config,
            start_time: Instant::now(),
            frame_count: 0,
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
            .with_title("Skiwave - Audio-Reactive Ocean")
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
            println!("\nSkiwave is running!");
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

        // Update camera position
        let (view_proj, camera_pos) = self
            .camera
            .create_view_proj_matrix(time_s, &self.render_config);

        // Update ocean simulation (returns modulated parameters)
        let (amplitude, frequency, line_width) =
            self.ocean.update(time_s, &audio_bands, camera_pos);

        // Grid stays at origin - camera flies over it (no tiling, just huge grid)
        let model = Mat4::IDENTITY;
        let mvp = view_proj * model;

        // Update ocean vertices
        render_system.update_vertices(&self.ocean.grid.vertices);

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
        if let Err(e) = render_system.render(self.frame_count) {
            eprintln!("Render error: {:?}", e);
        }

        self.frame_count += 1;
    }
}

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    println!("Skiwave - Fluid audio-reactive ocean surfing simulator");
    println!("Initializing systems...\n");

    // Parse camera preset
    let camera_preset = match args.camera_preset.to_lowercase().as_str() {
        "basic" => {
            println!("Camera: Basic (straight-line flight)");
            params::CameraPreset::Basic(params::BasicCameraPath::default())
        }
        "cinematic" => {
            println!("Camera: Cinematic (procedural journey)");
            params::CameraPreset::Cinematic(params::CameraJourney::default())
        }
        other => {
            eprintln!(
                "Warning: Unknown camera preset '{}', using cinematic",
                other
            );
            params::CameraPreset::Cinematic(params::CameraJourney::default())
        }
    };

    // Setup recording if requested
    let recording_config = args.record.map(|duration| {
        let config = RecordingConfig::new(duration);

        // Create output directories
        std::fs::create_dir_all(&config.frames_dir()).expect("Failed to create frames directory");
        std::fs::create_dir_all(&config.output_dir).expect("Failed to create output directory");

        config
    });

    let mut app = App::new(camera_preset, recording_config);
    let event_loop = EventLoop::new().unwrap();
    let _ = event_loop.run_app(&mut app);
}
