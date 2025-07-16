// extern
use anyhow::Result;
use glam;
use raw_window_handle::HasWindowHandle;
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

// intern
use crate::physics::{ModelType, PhysicsSystem};
use lavapond::{
    self, AnchorType, Renderer,
    camera::{ScreenPos2D, WorldPos2D},
};
use utils::input::{InputHandler, Inputs};

#[derive(Default)]
struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    physics_system: PhysicsSystem,
    inputs: Inputs,
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

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        match cause {
            StartCause::Init => {
                self.physics_system.arena(
                    glam::vec2(5.0, 5.0),
                    WorldPos2D::from_screen(&WINDOW_SIZE, 400., 400.),
                    glam::vec2(0.0, 0.0),
                    glam::vec3(0.05, 0.05, 0.05),
                );

                self.physics_system
                    .add_circle2(0.1, WorldPos2D::from_screen(&WINDOW_SIZE, 400., 400.));

                self.physics_system
                    .add_circle2(0.1, WorldPos2D::from_screen(&WINDOW_SIZE, 700., 700.));

                self.physics_system
                    .add_circle2(0.1, WorldPos2D::from_screen(&WINDOW_SIZE, 100., 100.));
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        //  Physics System
        self.physics_system.update();

        // Request Redraw
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.handle_inputs(&event);

        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested, stopping...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Create a renderer on the first request
                if self.renderer.is_none() {
                    if let Ok(renderer) = Renderer::new(&self.window.as_ref().unwrap()) {
                        println!(
                            "Renderer created with window handle: {:?}",
                            &self.window.as_ref().unwrap().window_handle().unwrap()
                        );
                        self.renderer = Some(renderer);
                    }
                    return;
                }

                // Draw Objects From Physics System Models
                for model in &self.physics_system.models {
                    match model.model_type {
                        ModelType::Circle(radius, color) => {
                            self.renderer.as_mut().unwrap().circle(
                                radius * 2.0,
                                model.position,
                                color.0,
                                AnchorType::Unlocked,
                            );
                        }
                        ModelType::Arena(x, y, color) => {
                            self.renderer.as_mut().unwrap().rectangle(
                                x,
                                y,
                                0.0,
                                model.position,
                                color.0,
                                AnchorType::Locked,
                            );
                        }
                    }
                }

                // Renderer
                self.renderer
                    .as_mut()
                    .unwrap()
                    .draw_request(&self.window.as_ref().unwrap());
            }
            WindowEvent::Resized(new_size) => {
                println!(
                    "Window resized... (w: {}, h: {})",
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

        // Create a new random sized circle at the mouse pos
        if self.inputs.just_pressed(Key::C) {
            if self.window.is_none() {
                return;
            }

            let width = self.window_width().unwrap();
            let height = self.window_height().unwrap();

            let pos = self
                .inputs
                .mouse_pos()
                .unwrap_or(glam::vec2(width / 2., height / 2.));

            self.physics_system.add_circle2(
                rand::random_range(0.1..0.5),
                WorldPos2D::from_screen2(&self.window_size().unwrap(), pos),
            );
        }
    }
}

impl App {
    pub fn window_width(&self) -> Option<f32> {
        if let Some(window) = &self.window {
            Some(window.inner_size().width as f32)
        } else {
            None
        }
    }

    pub fn window_height(&self) -> Option<f32> {
        if let Some(window) = &self.window {
            Some(window.inner_size().height as f32)
        } else {
            None
        }
    }

    pub fn window_size(&self) -> Option<PhysicalSize<u32>> {
        if let Some(window) = &self.window {
            Some(window.inner_size())
        } else {
            None
        }
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app);

    if app.renderer.is_some() {
        app.renderer.unwrap().wait_device_idle()?;
    }

    Ok(())
}
