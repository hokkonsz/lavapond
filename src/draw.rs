use crate::coord_sys::*;
use crate::resources;
use crate::shapes::*;
use anyhow::{Context, Result};
use ash::vk;
use std::time::{Duration, Instant};
use utils::color::Color;

impl super::Renderer {
    /* Drawing */

    /// Submits multiple draw commands to graphics queue based on the current `draw_pool` in
    ///
    /// 1. Fill `draw_pool` with objects to draw
    /// 2. Call `draw_request` function to submit draw
    /// 3. The `draw_pool` are cleared after submission
    pub fn draw_request(&mut self, window: &winit::window::Window) -> Result<()> {
        // Window Minimized -> No Draw
        if window.inner_size().height == 0 || window.inner_size().width == 0 {
            return Ok(());
        }

        /////////////////// STATISTICS DRAW ///////////////////

        self.text(
            &self.render_stats.as_text(),
            1.0,
            WorldPos2D::from_xy(&window.inner_size(), 5., 5.),
            AnchorType::Locked,
        )?;

        /////////////////// DRAW REQUEST TIMER ///////////////////
        self.render_stats.start_draw_request_timer();

        // Drawing
        unsafe {
            self.device.wait_for_fences(
                std::slice::from_ref(
                    self.fences_inflight
                        .get(self.current_frame)
                        .context("Inflight Fence: Index out of bounds")?,
                ),
                true,
                u64::MAX,
            )?;

            self.device.reset_fences(std::slice::from_ref(
                &self.fences_inflight[self.current_frame],
            ))?;

            let image_index = self
                .swapchain_loader
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    *self
                        .semaphores_acquire
                        .get(self.current_frame)
                        .context("Acquire Semaphore: Index out of bounds")?,
                    vk::Fence::null(),
                )?
                .0;

            self.device.reset_command_buffer(
                *self
                    .draw_command_buffers
                    .get(self.current_frame)
                    .context("Draw Command Buffer: Index out of bounds")?,
                vk::CommandBufferResetFlags::empty(),
            )?;

            self.device.begin_command_buffer(
                self.draw_command_buffers[self.current_frame],
                &vk::CommandBufferBeginInfo::default(),
            )?;

            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(
                    *self
                        .frame_buffers
                        .get(image_index as usize)
                        .context("Frame Buffer: Index out of bounds")?,
                )
                .render_area(self.scissor)
                .clear_values(&self.clear_color);

            self.device.cmd_begin_render_pass(
                self.draw_command_buffers[self.current_frame],
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );

            self.device.cmd_bind_pipeline(
                self.draw_command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );

            self.device.cmd_bind_vertex_buffers(
                self.draw_command_buffers[self.current_frame],
                0,
                &[self.vertex_buffer],
                &[0],
            );

            self.device.cmd_bind_index_buffer(
                self.draw_command_buffers[self.current_frame],
                self.index_buffer,
                0,
                vk::IndexType::UINT16,
            );

            self.device.cmd_set_viewport(
                self.draw_command_buffers[self.current_frame],
                0,
                std::slice::from_ref(&self.viewport),
            );

            self.device.cmd_set_scissor(
                self.draw_command_buffers[self.current_frame],
                0,
                std::slice::from_ref(&self.scissor),
            );

            let descriptor_set = self
                .descriptor_sets
                .get(self.current_frame)
                .context("Descriptor Sets: Index out of bounds")?;

            self.device.cmd_bind_descriptor_sets(
                self.draw_command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                std::slice::from_ref(descriptor_set),
                &[],
            );

            /////////////////// POOL CREATION TIMER START ///////////////////
            self.render_stats.start_pool_creation_timer();

            self.draw_from_pool()?;

            /////////////////// POOL CREATION TIMER STOP ///////////////////
            self.render_stats.stop_pool_creation_timer();

            self.device
                .cmd_end_render_pass(self.draw_command_buffers[self.current_frame]);

            self.device
                .end_command_buffer(self.draw_command_buffers[self.current_frame])?;

            self.camera.update_projection(window);

            let mut uniform_align = ash::util::Align::new(
                *self
                    .uniform_buffers_mapped
                    .get(self.current_frame)
                    .context("Uniform Buffers Mapped: Index out of bounds")?,
                std::mem::align_of::<u16>() as u64,
                self.uniform_buffers_mem_req
                    .get(self.current_frame)
                    .context("Uniform Buffers Mem Req: Index out of bounds")?
                    .size,
            );

            uniform_align.copy_from_slice(&std::slice::from_ref(self.camera.get_view_projection()));

            let submit_info = vk::SubmitInfo::default()
                .wait_dst_stage_mask(std::slice::from_ref(
                    &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ))
                .wait_semaphores(std::slice::from_ref(
                    &self.semaphores_acquire[self.current_frame],
                ))
                .command_buffers(std::slice::from_ref(
                    &self.draw_command_buffers[self.current_frame],
                ))
                .signal_semaphores(std::slice::from_ref(
                    self.semaphores_release
                        .get(self.current_frame)
                        .context("Release Semaphores: Index out of bounds")?,
                ));

            self.device.queue_submit(
                self.graphics_queue,
                std::slice::from_ref(&submit_info),
                self.fences_inflight[self.current_frame],
            )?;

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(std::slice::from_ref(
                    &self.semaphores_release[self.current_frame],
                ))
                .swapchains(std::slice::from_ref(&self.swapchain))
                .image_indices(std::slice::from_ref(&image_index));

            self.swapchain_loader
                .queue_present(self.present_queue, &present_info)?;

            let frame = (self.current_frame + 1) % Self::MAX_FRAMES_INFLIGHT;
            self.current_frame = frame;
        }

        /////////////////// DRAW REQUEST TIMER START ///////////////////
        self.render_stats.stop_draw_request_timer();

        /////////////////// UPDATE STATISTICS ///////////////////
        self.update_render_stats();

        // Reset Draw Pool
        self.draw_pool.clear();

        Ok(())
    }

    /// For each `draw_instance` in the [`Renderer`]'s `draw_pool`
    /// * Creates an a transformation matrix based on the instance's position, rototation and scale
    /// * Adds a push constant
    /// * Adds an indexed draw command
    ///
    /// Used only internally by draw_request function!
    fn draw_from_pool(&mut self) -> Result<()> {
        let mut draw_instance_data = DrawInstanceData::default();

        for draw_instance in &self.draw_pool {
            draw_instance_data.transform = glam::Mat4::from_scale_rotation_translation(
                draw_instance.scale,
                glam::Quat::from_rotation_z(draw_instance.rotation.to_radians()),
                draw_instance.position,
            );

            draw_instance_data.color = draw_instance.color;

            unsafe {
                self.device.cmd_push_constants(
                    self.draw_command_buffers[self.current_frame],
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    &bytemuck::try_cast_slice(draw_instance_data.as_slice())?,
                );

                self.device.cmd_draw_indexed(
                    self.draw_command_buffers[self.current_frame],
                    self.object_pool.pool[draw_instance.object_index].index_count as u32,
                    1,
                    self.object_pool.pool[draw_instance.object_index].index_offset as u32,
                    0,
                    0,
                );
            }
        }

        Ok(())
    }

    /* Creating Draw Instances */

    /// Adds the characters of the text to the instance pool
    pub fn text(
        &mut self,
        text: &str,
        scale: f32,
        top_left: WorldPos2D,
        anchor_type: AnchorType,
    ) -> Result<()> {
        // let scale = scale * self.scene.camera_zoom;
        let pad_x = scale * 0.03;
        let pad_y = scale * 0.05;

        let anchor_position = match anchor_type {
            AnchorType::Locked => glam::vec3(
                top_left.x + self.camera.get_position().x + pad_x,
                top_left.y + self.camera.get_position().y - pad_y,
                0.0,
            ),
            AnchorType::Unlocked => glam::vec3(top_left.x + pad_x, top_left.y - pad_y, 0.0),
        };

        let mut char_index;
        let mut text_instance_pool = Vec::with_capacity(text.len());
        let mut cursor_position = anchor_position;

        for byte in text.bytes() {
            char_index = resources::CHAR_OBJECT_POOL[byte as usize];

            // There are no corresponding character object
            if char_index == 255 {
                continue;
            };

            // Move the cursor to the next line
            if char_index == 253 {
                cursor_position.x = anchor_position.x;
                cursor_position.y -= pad_y;
                continue;
            };

            // Add the current char to the draw pool
            if char_index != 254 {
                text_instance_pool.push(DrawInstance {
                    position: cursor_position,
                    scale: glam::vec3(scale, scale, 0.0),
                    object_index: char_index as usize,
                    color: Color::EMERALD,
                    ..DrawInstance::default()
                });
            }

            // Move the cursor by 1 character to right
            cursor_position.x += pad_x;
        }

        self.draw_pool.extend(text_instance_pool);

        Ok(())
    }

    /// Adds a shape to the instance pool
    pub fn shape(
        &mut self,
        size_x: f32,
        size_y: f32,
        rotation: f32,
        center: WorldPos2D,
        color: Color,
        shape: &ShapeType,
        anchor_type: AnchorType,
    ) -> Result<()> {
        let position = match anchor_type {
            AnchorType::Locked => glam::vec3(
                center.x + self.camera.get_position().x,
                center.y + self.camera.get_position().y,
                0.0,
            ),
            AnchorType::Unlocked => glam::vec3(center.x, center.y, 0.0),
        };

        let object_index = match shape {
            ShapeType::Circle => resources::ObjectPool::CIRCLE,
            ShapeType::CircleBorder => resources::ObjectPool::CIRCLE_BORDER,
            ShapeType::Rectangle => resources::ObjectPool::RECTANGLE,
            ShapeType::RectangleBorder => resources::ObjectPool::RECTANGLE_BORDER,
            ShapeType::RoundedRectangle => resources::ObjectPool::ROUNDED_RECTANGLE,
            ShapeType::RoundedRectangleBorder => resources::ObjectPool::ROUNDED_RECTANGLE_BORDER,
        };

        self.draw_pool.push(DrawInstance {
            position,
            rotation,
            scale: glam::vec3(size_x, size_y, 0.0),
            color,
            object_index,
        });

        Ok(())
    }

    /* Render Statistics */

    /// Updates the render statistics structure based on the time elapsed
    fn update_render_stats(&mut self) -> () {
        if self.render_stats.turned_off {
            return;
        }

        // Update Frame Counter
        if self.render_stats.fps_instant.elapsed() >= Duration::from_secs(1) {
            self.render_stats.frames_per_sec = self.render_stats.frame_counter;

            self.render_stats.frame_counter = 0;
            self.render_stats.fps_instant = Instant::now();
        } else {
            self.render_stats.frame_counter += 1;
        }

        // Update Pool Stats
        if self.render_stats.last_draw_pool_elements != self.draw_pool.len() {
            self.render_stats.last_draw_pool_elements = self.draw_pool.len();
        }

        if self.render_stats.last_draw_pool_vertices != self.object_pool.vertices.len() {
            self.render_stats.last_draw_pool_vertices = self.object_pool.vertices.len();
        }
    }

    pub fn add_shape(&mut self, shape: &impl Shape, anchor: AnchorType) -> Result<()> {
        let draw_params = shape.get_drawparams();

        self.shape(
            draw_params.size_x(),
            draw_params.size_y(),
            draw_params.rotation(),
            draw_params.center(),
            draw_params.color(),
            draw_params.shape_type(),
            anchor,
        )
    }
}

//==================================================
//=== Render Statistics
//==================================================

pub struct RenderStats {
    turned_off: bool,
    frames_per_sec: u32,
    last_draw_request_time: u128,
    last_draw_pool_creation_time: u128,
    last_draw_pool_elements: usize,
    last_draw_pool_vertices: usize,
    frame_counter: u32,
    fps_instant: Instant,
    draw_request_instant: Instant,
    pool_creation_instant: Instant,
}

impl RenderStats {
    /// Creates a new render statistics
    pub fn new() -> Self {
        Self {
            turned_off: false,
            frames_per_sec: 0,
            last_draw_request_time: 0,
            last_draw_pool_creation_time: 0,
            last_draw_pool_elements: 0,
            last_draw_pool_vertices: 0,
            frame_counter: 0,
            fps_instant: Instant::now(),
            draw_request_instant: Instant::now(),
            pool_creation_instant: Instant::now(),
        }
    }

    /// Starts the timer of draw request
    fn start_draw_request_timer(&mut self) -> () {
        if self.turned_off {
            return;
        }

        self.draw_request_instant = Instant::now();
    }

    /// Stops the timer of draw request
    fn stop_draw_request_timer(&mut self) -> () {
        if self.turned_off {
            return;
        }

        self.last_draw_request_time = self.draw_request_instant.elapsed().as_micros();
    }

    /// Starts the timer of pool creation
    fn start_pool_creation_timer(&mut self) -> () {
        if self.turned_off {
            return;
        }

        self.pool_creation_instant = Instant::now();
    }

    /// Stops the timer of pool creation
    fn stop_pool_creation_timer(&mut self) -> () {
        if self.turned_off {
            return;
        }

        self.last_draw_pool_creation_time = self.pool_creation_instant.elapsed().as_micros();
    }

    /// Gives back the current stats as a [`String`]
    fn as_text(&self) -> String {
        format!(
            "fps: {}\nrequest time: {} us\npool creation time:{}\nelements:{}\nvertices:{}",
            self.frames_per_sec,
            self.last_draw_request_time,
            self.last_draw_pool_creation_time,
            self.last_draw_pool_elements,
            self.last_draw_pool_vertices
        )
    }
}

//==================================================
//=== Draw Instance
//==================================================

/// Locked = Object moves with the camera
///
/// Unlocked = Object does not moves with the camera
pub enum AnchorType {
    Locked,
    Unlocked,
}

#[derive(Default)]
pub struct DrawInstanceData {
    transform: glam::Mat4,
    color: Color,
}

impl DrawInstanceData {
    const TRANSFORM_LEN: usize = 4 * 4;
    const COLOR_LEN: usize = 3;

    /// Gives back the [`DrawInstanceData`] as a slice
    ///
    /// # Safety
    ///
    /// This is safe to call, since the safety conditions
    /// of the`std::slice::from_raw_parts` function are met.
    pub fn as_slice(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                self.transform.as_ref().as_ptr(),
                Self::TRANSFORM_LEN + Self::COLOR_LEN,
            )
        }
    }
}

#[derive(Clone, Default)]
pub struct DrawInstance {
    pub position: glam::Vec3,
    pub rotation: f32,
    pub scale: glam::Vec3,
    pub color: Color,
    pub object_index: usize,
}

impl DrawInstance {
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}
