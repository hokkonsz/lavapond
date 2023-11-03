#![allow(unused)]

use std::time::Instant;

// Extern
use anyhow::Result;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{CursorIcon, WindowBuilder},
};

const WINDOW_HEIGHT: u32 = 600;
const WINDOW_WIDTH: u32 = 800;

// Intern
use crate::physics::{ModelType, PhysicsSystem};
use crate::vulkan::{self, AnchorType, Renderer};

/// Runs application
pub fn run() -> Result<()> {
    // Window
    let event_loop = EventLoop::new();

    let mut window_size = PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let window = WindowBuilder::new()
        .with_title("lavapond")
        .with_inner_size(window_size)
        .build(&event_loop)?;

    // Input Handling
    let mut lmb_down = false;
    let mut last_mouse_pos: Option<PhysicalPosition<f64>> = None;
    let mut mouse_pos: PhysicalPosition<f64> = PhysicalPosition::new(0.0, 0.0);
    let mut center_pos: PhysicalPosition<f64> = PhysicalPosition::new(0.0, 0.0);

    // Physics System
    let mut physics_system = PhysicsSystem::new();

    // Vulkan Renderer
    let mut renderer = Renderer::new(&window)?;
    let mut res: Result<()> = Ok(());

    ///////////////// DEBUG /////////////////
    let mut last_creation_pos: PhysicalPosition<f64> = PhysicalPosition::new(0.0, 0.0);

    physics_system.circle(0.1, 0.0, 0.0, 0.0, 0.0);
    physics_system.circle(0.1, -1.0, -1.0, 0.0, 0.0);
    physics_system.circle(0.1, 1.0, 1.0, 0.0, 0.0);
    ///////////////// DEBUG /////////////////

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();
        match event {
            Event::MainEventsCleared => {
                // Physics System
                physics_system.update();

                // Draw Objects From Physics System Models
                for model in &physics_system.models {
                    match model.model_type {
                        ModelType::Circle(r) => {
                            renderer.circle(
                                r * 2.0,
                                model.position.x,
                                model.position.y,
                                AnchorType::Unlocked,
                            );
                        }
                        ModelType::Rectangle(a, b) => {
                            renderer.rectangle(
                                1.0,
                                model.position.x,
                                model.position.y,
                                AnchorType::Locked,
                            );
                        }
                        ModelType::Arena(a, b) => {
                            renderer.rectangle(
                                1.0,
                                model.position.x,
                                model.position.y,
                                AnchorType::Locked,
                            );
                        }
                    }
                }

                // Renderer
                res = control_flow.check_result(renderer.draw_request(&window));
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => control_flow.set_exit(),
                WindowEvent::Resized(new_size) => {
                    if new_size == window.inner_size() {
                        window_size = new_size;
                        res = control_flow.check_result(renderer.recreate_swapchain(new_size));
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(key) = input.virtual_keycode {
                        match key {
                            VirtualKeyCode::C if input.state == ElementState::Released => {
                                let window_width = window_size.width as f64;
                                let window_height = window_size.height as f64;

                                let x = ((2.0 * (mouse_pos.x - center_pos.x) - window_width)
                                    / window_width) as f32;
                                let y = -((2.0 * (mouse_pos.y - center_pos.y) - window_height)
                                    / window_height) as f32;

                                physics_system.circle(0.1, x as f32, y as f32, 0.0, 0.0);
                            }
                            _ => (),
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let winit::event::MouseScrollDelta::LineDelta(_, dir) = delta {
                        renderer.scene.zoom(dir * 0.1);
                    }
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    if let MouseButton::Left = button {
                        match state {
                            ElementState::Pressed => {
                                lmb_down = true;
                                window.set_cursor_icon(CursorIcon::Grabbing)
                            }
                            ElementState::Released => {
                                lmb_down = false;
                                window.set_cursor_icon(CursorIcon::Default);
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    mouse_pos = position;

                    if lmb_down {
                        if let Some(last_position) = last_mouse_pos {
                            let window_width = window_size.width as f64;
                            let window_height = window_size.height as f64;

                            let x = ((last_position.x - mouse_pos.x) / window_width) as f32;
                            let y = ((last_position.y - mouse_pos.y) / window_height) as f32;

                            renderer.scene.pan_view_xy(x, y);
                            center_pos.x += x as f64;
                            center_pos.y += y as f64;
                        }

                        last_mouse_pos = Some(position);
                    } else {
                        last_mouse_pos = None;
                    }
                }

                _ => (),
            },
            // Event::DeviceEvent { event, .. } => match event {
            //     DeviceEvent::MouseMotion { delta } => {}
            //     _ => (),
            // },
            _ => (),
        }
    });

    res
}

trait EventResult {
    fn check_result(&mut self, result: Result<()>) -> Result<()> {
        Ok(())
    }
}

impl EventResult for ControlFlow {
    fn check_result(&mut self, result: Result<()>) -> Result<()> {
        if let Err(e) = result {
            self.set_exit();
            return Err(e);
        }

        Ok(())
    }
}
