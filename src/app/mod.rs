#![allow(unused)]

// Extern
use anyhow::Result;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{CursorIcon, WindowBuilder},
};

// Intern
use crate::vulkan::Renderer;

mod ui_elements;

use ui_elements::*;

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Triangle")
        .with_inner_size(PhysicalSize::new(800u32, 600))
        .build(&event_loop)?;

    let mut renderer = Renderer::new(&window)?;

    let mut res: Result<()> = Ok(());
    let mut lmb_down = false;
    let mut last_mouse_pos: Option<PhysicalPosition<f64>> = None;

    // Statistics
    let mut frame_counter = FrameCounter::new();
    let mut fps_text = String::from("FPS: -");

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            Event::MainEventsCleared => {
                frame_counter.count();

                if frame_counter.changed() {
                    renderer.draw_pool =
                        renderer.text_box(&format!("FPS: {}", frame_counter.last_frame_count()));
                }

                res = control_flow.check_result(renderer.draw_request(&window));
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => control_flow.set_exit(),
                WindowEvent::Resized(new_size) => {
                    if new_size == window.inner_size() {
                        res = control_flow.check_result(renderer.recreate_swapchain(new_size));
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(key) = input.virtual_keycode {
                        match key {
                            // Arrows
                            // VirtualKeyCode::Up => renderer.object_position.y += 0.001,
                            // VirtualKeyCode::Down => renderer.object_position.y -= 0.001,
                            // VirtualKeyCode::Left => renderer.object_position.x -= 0.001,
                            // VirtualKeyCode::Right => renderer.object_position.x += 0.001,
                            // VirtualKeyCode::Space => window.request_redraw(),
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
                                window.set_cursor_icon(CursorIcon::Hand)
                            }
                            ElementState::Released => {
                                lmb_down = false;
                                window.set_cursor_icon(CursorIcon::Default);
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if lmb_down {
                        if let Some(last_position) = last_mouse_pos {
                            let aspect = (window.inner_size().width as f32)
                                / (window.inner_size().height as f32);

                            renderer.scene.pan_view_xy(
                                ((last_position.x - position.x) / window.scale_factor()) as f32
                                    * 0.002,
                                ((last_position.y - position.y) / window.scale_factor()) as f32
                                    * 0.002,
                            );
                        }

                        last_mouse_pos = Some(position);
                    } else {
                        last_mouse_pos = None;
                    }
                }

                _ => (),
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    // cam_pos.x += (delta.0 * 0.01) as f32;
                    // cam_pos.y += (delta.1 * 0.01) as f32;
                }
                _ => (),
            },
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
