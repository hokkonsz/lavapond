use crate::bezier::Bezier;
use anyhow::Result;
use lavapond::AnchorType;
use lavapond::shapes::ShapeType;
use lavapond::{self, Renderer, coord_sys::WorldPos2D};
use raw_window_handle::HasWindowHandle;
use utils::color::Color;
use utils::input::{InputHandler, Inputs};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

const WINDOW_SIZE: PhysicalSize<u32> = PhysicalSize {
    width: 600,
    height: 600,
};

const AXIS_LENGTH: f32 = 1.0;
const AXIS_THICKNESS: f32 = 0.01;

#[derive(Default)]
struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
    bezier: Bezier,
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
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
            self.handle_inputs(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested, stopping...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let window = self.window.as_ref().unwrap();

                    // Axis X
                    renderer.shape(
                        AXIS_LENGTH,
                        AXIS_THICKNESS,
                        0.0,
                        WorldPos2D::new(0.0, -AXIS_LENGTH / 2.),
                        Color::ONYX,
                        &ShapeType::Rectangle,
                        AnchorType::Unlocked,
                    );

                    // Axis Y
                    renderer.shape(
                        AXIS_THICKNESS,
                        AXIS_LENGTH,
                        0.0,
                        WorldPos2D::new(-AXIS_LENGTH / 2., 0.0),
                        Color::ONYX,
                        &ShapeType::Rectangle,
                        AnchorType::Unlocked,
                    );

                    // Lines
                    let x = (self.bezier.control_points[1].x) - (self.bezier.control_points[0].x);
                    let y = (self.bezier.control_points[1].y) - (self.bezier.control_points[0].y);
                    let length = (x * x + y * y).sqrt();
                    let rotation = (y / x).atan().to_degrees();
                    renderer.shape(
                        length,
                        0.005,
                        rotation,
                        WorldPos2D::new(
                            x / 2. + self.bezier.control_points[0].x - AXIS_LENGTH / 2.,
                            y / 2. + self.bezier.control_points[0].y - AXIS_LENGTH / 2.,
                        ),
                        Color::RED,
                        &ShapeType::Rectangle,
                        AnchorType::Unlocked,
                    );

                    // Line Control Points 3-4
                    let x = (self.bezier.control_points[2].x) - (self.bezier.control_points[3].x);
                    let y = (self.bezier.control_points[2].y) - (self.bezier.control_points[3].y);
                    let length = (x * x + y * y).sqrt();
                    let rotation = (y / x).atan().to_degrees();
                    renderer.shape(
                        length,
                        0.005,
                        rotation,
                        WorldPos2D::new(
                            x / 2. + self.bezier.control_points[3].x - AXIS_LENGTH / 2.,
                            y / 2. + self.bezier.control_points[3].y - AXIS_LENGTH / 2.,
                        ),
                        Color::RED,
                        &ShapeType::Rectangle,
                        AnchorType::Unlocked,
                    );
                    // Curve
                    for i in 0..self.bezier.resolution {
                        // https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Cubic_B%C3%A9zier_curves
                        // Point(t) = it^3*CP0 + 3*it^2*t*CP1 + 3*it*t^2*CP2 + t^3*CP3
                        let t = i as f32 / self.bezier.resolution as f32;
                        let it = 1. - t;

                        #[rustfmt::skip]
                         let point = it * it * it * self.bezier.control_points[0] +
                                3. * it * it *  t * self.bezier.control_points[1] +
                                3. * it *  t *  t * self.bezier.control_points[2] +
                                      t *  t *  t * self.bezier.control_points[3];

                        renderer.shape(
                            Bezier::CONTROL_POINT_SIZE - 0.01,
                            Bezier::CONTROL_POINT_SIZE - 0.01,
                            0.0,
                            WorldPos2D::new(point.x - AXIS_LENGTH / 2., point.y - AXIS_LENGTH / 2.),
                            Color::EMERALD,
                            &ShapeType::Circle,
                            AnchorType::Unlocked,
                        );

                        // Control point
                        for (i, cp) in self.bezier.control_points.iter().enumerate() {
                            renderer.shape(
                                Bezier::CONTROL_POINT_SIZE,
                                Bezier::CONTROL_POINT_SIZE,
                                0.0,
                                WorldPos2D::new(cp.x - AXIS_LENGTH / 2., cp.y - AXIS_LENGTH / 2.),
                                self.bezier.control_points_color[i],
                                &ShapeType::Circle,
                                AnchorType::Unlocked,
                            );
                        }
                    }

                    // Send redraw request to renderer
                    renderer.draw_request(&window);

                // Create a renderer on the first request
                } else {
                    match Renderer::new(&self.window.as_ref().unwrap(), Color::MIDNIGHT) {
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
                }
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
        self.inputs.read(event);

        let window_size = self.window.as_ref().unwrap().inner_size();

        // Create a new random sized circle at the mouse pos
        if let Some(mouse_pos) = self.inputs.mouse_pos() {
            // Convert mouse screen coordinates to world coordinates
            let mut mouse_pos = WorldPos2D::from_xy(&window_size, mouse_pos.x, mouse_pos.y);
            // Offset mouse pos
            mouse_pos.x += AXIS_LENGTH / 2.;
            mouse_pos.y += AXIS_LENGTH / 2.;
            if let Some(i) = self.bezier.dragged_point {
                self.bezier.control_points[i].x = mouse_pos.x;
                self.bezier.control_points[i].y = mouse_pos.y;
                if self.inputs.lmb_just_released() {
                    self.bezier.dragged_point = None;
                }
            } else {
                for (i, cp) in self.bezier.control_points.iter().enumerate() {
                    let dist_x = mouse_pos.x - cp.x;
                    let dist_y = mouse_pos.y - cp.y;
                    let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();
                    let hovered = dist <= Bezier::CONTROL_POINT_SIZE;
                    if hovered {
                        self.bezier.control_points_color[i] = Color::RED;
                        if self.inputs.lmb_just_pressed() {
                            self.bezier.dragged_point = Some(i);
                        }
                    } else {
                        if i < 2 {
                            self.bezier.control_points_color[i] = Color::YELLOW;
                        } else {
                            self.bezier.control_points_color[i] = Color::ROYAL_BLUE;
                        }
                    }
                }
            }
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
