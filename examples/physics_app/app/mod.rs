use crate::physics::PhysicsSystem;
use anyhow::Result;
use glam;
use lavapond::AnchorType;
use lavapond::{self, Renderer, coord_sys::WorldPos2D};
use raw_window_handle::HasWindowHandle;
use utils::input::{InputHandler, Inputs};
use utils::timer::Timer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

const WINDOW_SIZE: PhysicalSize<u32> = PhysicalSize {
    width: 800,
    height: 600,
};

#[derive(Default)]
struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    physics_system: PhysicsSystem,
    timer: Timer,
    inputs: Inputs,
    error: Option<anyhow::Error>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(WINDOW_SIZE)
                        .with_title("lavapond"),
                )
                .unwrap(),
        );
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::Init => {
                self.timer.set_hz(10);
                self.physics_system
                    .bounding_box(WorldPos2D::new(0.0, 0.0), 0.0, 1.0, 1.0);
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        //  Physics System
        self.physics_system.update();

        // Request Redraw
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.error.is_some() {
            event_loop.exit();
            return;
        }

        if self.window.is_some() {
            // self.handle_inputs(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested, stopping...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Create a renderer on the first request
                if self.renderer.is_none() {
                    match Renderer::new(&self.window.as_ref().unwrap()) {
                        Ok(renderer) => {
                            println!(
                                "Renderer created with window handle: {:?}",
                                &self.window.as_ref().unwrap().window_handle().unwrap()
                            );
                            self.renderer = Some(renderer);
                        }
                        Err(err) => {
                            self.error = Some(err);
                        }
                    }

                    // Initialize physics system after renderer is created
                    self.physics_system
                        .add_circle2(0.1, WorldPos2D::from_xy(&WINDOW_SIZE, 400., 300.));

                    self.physics_system
                        .add_circle2(0.1, WorldPos2D::from_xy(&WINDOW_SIZE, 700., 75.));

                    self.physics_system
                        .add_circle2(0.1, WorldPos2D::from_xy(&WINDOW_SIZE, 100., 525.));

                    return;
                }

                // Draw Objects From Physics System Models
                let renderer = self.renderer.as_mut().unwrap();
                let bb = &self.physics_system.bounding_box;
                renderer.add_shape(bb, AnchorType::Unlocked);
                for model in &self.physics_system.models {
                    renderer.add_shape(model, AnchorType::Unlocked);
                }

                // Renderer
                self.renderer
                    .as_mut()
                    .unwrap()
                    .draw_request(&self.window.as_ref().unwrap());
            }
            WindowEvent::Resized(new_size) => {
                println!(
                    "Window resized: (w: {}, h: {})",
                    new_size.width, new_size.height
                );

                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.recreate_swapchain(new_size);
                }
            }
            _ => (),
        }
    }
}

impl InputHandler for App {
    fn handle_inputs(&mut self, event: &winit::event::WindowEvent) {
        use utils::input::Key;

        self.inputs.read(event);

        // Start / Stop Physics System
        if self.inputs.just_pressed(Key::Space) {
            self.physics_system.switch_state();
        }

        if self.window.is_none() {
            return;
        }
        let window = self.window.as_ref().unwrap();
        let window_size = window.inner_size();

        // Create a new random sized circle at the mouse pos
        if self.inputs.just_pressed(Key::C) {
            let pos = self.inputs.mouse_pos().unwrap_or(glam::vec2(
                window_size.width as f32 / 2.,
                window_size.height as f32 / 2.,
            ));

            self.physics_system.add_circle2(
                rand::random_range(0.1..0.5),
                WorldPos2D::from_vec2(&window_size, pos),
            );
        }

        if self.renderer.is_none() || !self.timer.is_repeating() {
            return;
        }
        let renderer = self.renderer.as_mut().unwrap();

        // Move Camera
        if self.inputs.held_down(Key::W) {
            renderer.camera.shift(WorldPos2D::new(0.0, -0.01));
        } else if self.inputs.just_pressed(Key::S) {
            renderer.camera.shift(WorldPos2D::new(0.0, 0.01));
        }

        if self.inputs.just_pressed(Key::A) {
            renderer.camera.shift(WorldPos2D::new(0.01, 0.0));
        } else if self.inputs.just_pressed(Key::D) {
            renderer.camera.shift(WorldPos2D::new(-0.01, 0.0));
        }
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    if let Some(renderer) = app.renderer {
        renderer.wait_device_idle()?;
    }

    if let Some(error) = app.error {
        return Result::Err(error);
    }

    Ok(())
}
