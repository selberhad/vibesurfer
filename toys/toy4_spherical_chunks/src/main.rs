use std::sync::Arc;
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

// === Configuration ===

const PLANET_RADIUS: f32 = 1_000_000.0; // 1000km radius
const CHUNK_SIZE: u32 = 256; // 256x256 vertices per chunk
const DEFAULT_ALTITUDE: f32 = 100.0; // 100m above surface
const DEFAULT_SPEED: f32 = 100.0; // 100 m/s tangential velocity
const DEFAULT_SPACING: f32 = 2.0; // 2m between vertices

// === Orbital Camera ===

struct OrbitCamera {
    altitude: f32,         // Height above surface (m)
    angular_pos: f32,      // Current angle around sphere (radians)
    angular_velocity: f32, // Orbital speed (rad/s)
    paused: bool,
}

impl OrbitCamera {
    fn new(altitude: f32, speed_m_s: f32) -> Self {
        // Convert linear speed to angular velocity: v = r * ω
        let r = PLANET_RADIUS + altitude;
        let angular_velocity = speed_m_s / r;

        Self {
            altitude,
            angular_pos: 0.0,
            angular_velocity,
            paused: false,
        }
    }

    fn update(&mut self, dt: f32) {
        if !self.paused {
            self.angular_pos += self.angular_velocity * dt;
            self.angular_pos %= std::f32::consts::TAU; // Wrap at 2π
        }
    }

    fn position(&self) -> [f32; 3] {
        // Equatorial orbit (latitude = 0)
        let r = PLANET_RADIUS + self.altitude;
        let lat: f32 = 0.0;
        let lon = self.angular_pos;

        // Spherical to Cartesian
        [
            r * lat.cos() * lon.cos(),
            r * lat.sin(),
            r * lat.cos() * lon.sin(),
        ]
    }

    fn adjust_altitude(&mut self, delta: f32) {
        self.altitude = (self.altitude + delta).max(1.0); // Min 1m altitude
        println!(
            "Altitude: {:.1}m (radius: {:.1}m)",
            self.altitude,
            PLANET_RADIUS + self.altitude
        );
    }

    fn adjust_speed(&mut self, delta_m_s: f32) {
        let current_speed = self.angular_velocity * (PLANET_RADIUS + self.altitude);
        let new_speed = (current_speed + delta_m_s).max(1.0);
        self.angular_velocity = new_speed / (PLANET_RADIUS + self.altitude);
        println!("Orbital speed: {:.1} m/s", new_speed);
    }
}

// === Chunk System ===

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
struct ChunkId {
    lat_cell: i32,
    lon_cell: i32,
}

impl ChunkId {
    fn from_position(position: [f32; 3], chunk_size_radians: f32) -> Self {
        // Convert XYZ to lat/lon
        let x = position[0];
        let y = position[1];
        let z = position[2];

        let r = (x * x + y * y + z * z).sqrt();
        let lat = (y / r).asin();
        let lon = z.atan2(x);

        ChunkId {
            lat_cell: (lat / chunk_size_radians).floor() as i32,
            lon_cell: (lon / chunk_size_radians).floor() as i32,
        }
    }
}

// === App State ===

struct App {
    camera: OrbitCamera,
    grid_spacing: f32,
    last_frame: std::time::Instant,
    frame_count: u32,
    fps_timer: std::time::Instant,
}

impl App {
    fn new() -> Self {
        Self {
            camera: OrbitCamera::new(DEFAULT_ALTITUDE, DEFAULT_SPEED),
            grid_spacing: DEFAULT_SPACING,
            last_frame: std::time::Instant::now(),
            frame_count: 0,
            fps_timer: std::time::Instant::now(),
        }
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        self.camera.update(dt);

        // FPS counter
        self.frame_count += 1;
        if (now - self.fps_timer).as_secs() >= 1 {
            println!("FPS: {}", self.frame_count);
            self.frame_count = 0;
            self.fps_timer = now;
        }
    }

    fn handle_input(&mut self, keycode: KeyCode) {
        match keycode {
            KeyCode::Digit1 => self.camera.adjust_altitude(10.0),
            KeyCode::Digit2 => self.camera.adjust_altitude(-10.0),
            KeyCode::Digit3 => self.camera.adjust_speed(10.0),
            KeyCode::Digit4 => self.camera.adjust_speed(-10.0),
            KeyCode::Digit5 => {
                self.grid_spacing = (self.grid_spacing * 0.5).max(0.25);
                println!("Grid spacing: {:.2}m", self.grid_spacing);
            }
            KeyCode::Digit6 => {
                self.grid_spacing = (self.grid_spacing * 2.0).min(10.0);
                println!("Grid spacing: {:.2}m", self.grid_spacing);
            }
            KeyCode::Space => {
                self.camera.paused = !self.camera.paused;
                println!(
                    "Orbit {}",
                    if self.camera.paused {
                        "paused"
                    } else {
                        "resumed"
                    }
                );
            }
            KeyCode::KeyP => {
                let pos = self.camera.position();
                println!(
                    "Camera: altitude={:.1}m, pos=[{:.1}, {:.1}, {:.1}], angle={:.3}rad",
                    self.camera.altitude, pos[0], pos[1], pos[2], self.camera.angular_pos
                );
            }
            _ => {}
        }
    }
}

// === Main ===

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(
        event_loop
            .create_window(
                winit::window::Window::default_attributes()
                    .with_title("Toy 4: Spherical Chunk Streaming")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
            )
            .unwrap(),
    );

    let mut app = App::new();

    println!("=== Toy 4: Spherical Chunk Streaming ===");
    println!(
        "Planet radius: {}m ({:.1}km)",
        PLANET_RADIUS,
        PLANET_RADIUS / 1000.0
    );
    println!("Chunk size: {}x{} vertices", CHUNK_SIZE, CHUNK_SIZE);
    println!("\nControls:");
    println!("  1/2 - Adjust altitude");
    println!("  3/4 - Adjust speed");
    println!("  5/6 - Adjust grid spacing");
    println!("  Space - Pause/resume orbit");
    println!("  P - Print camera stats");
    println!();

    let _ = event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(keycode),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        app.handle_input(keycode);
                    }
                    WindowEvent::RedrawRequested => {
                        app.update();
                        // TODO: Render frame
                        window.request_redraw();
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}
