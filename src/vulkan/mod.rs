#![allow(unused_mut)]

// std
use std::{
    ffi::CStr,
    time::{Duration, Instant},
};

// extern
extern crate nalgebra_glm as glm;
use anyhow::{anyhow, Context, Result};
use ash::{
    extensions::{ext, khr},
    util,
    vk::{self, DescriptorSet},
};
use raw_window_handle::HasRawDisplayHandle;
use winit::dpi::PhysicalSize;

// intern
mod buffers;
mod descriptor;
mod extensions;
mod pipeline;
mod resources;

use buffers::*;
use descriptor::*;
use extensions::*;
use pipeline::*;
use resources::*;

//==================================================
//=== Renderer
//==================================================

pub struct Renderer {
    // Vulkan: Base
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    image_views: Vec<vk::ImageView>,

    // Vulkan: Extensions
    debug_utils_loader: Option<ext::DebugUtils>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    surface_loader: khr::Surface,
    surface: vk::SurfaceKHR,
    swapchain_loader: khr::Swapchain,
    swapchain: vk::SwapchainKHR,

    // Vulkan: Descriptor
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<DescriptorSet>,

    // Vulkan: Graphics Pipeline
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    graphics_pipeline: vk::Pipeline,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    viewport: vk::Viewport,
    scissor: vk::Rect2D,
    push_constant_range: vk::PushConstantRange,

    // Vulkan: Buffers
    frame_buffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    draw_command_buffers: Vec<vk::CommandBuffer>,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffers_memory: Vec<vk::DeviceMemory>,
    uniform_buffers_mem_req: Vec<vk::MemoryRequirements>,
    uniform_buffers_mapped: Vec<*mut std::ffi::c_void>,

    // Vulkan: Syncronization
    semaphores_acquire: Vec<vk::Semaphore>,
    semaphores_release: Vec<vk::Semaphore>,
    fences_inflight: Vec<vk::Fence>,

    // Render Loop Data
    current_frame: usize,
    pub scene: Scene,
    object_pool: ObjectPool,
    pub draw_pool: Vec<ObjectInstance>,
    render_stats: RenderStats,
}

impl Renderer {
    const MAX_FRAMES_INFLIGHT: usize = 2;

    const CLEAR_VALUES: [vk::ClearValue; 1] = [vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    }];

    /// Creates a new [`Renderer`] using `window`
    pub fn new(window: &winit::window::Window) -> Result<Renderer> {
        // Pre Load Object Pool
        let object_pool = resources::preload()?;

        let window_size =
            winit::dpi::PhysicalSize::new(window.inner_size().width, window.inner_size().height);

        // Base: Entry & Instance
        let entry = unsafe { ash::Entry::load() }?;

        let instance = create_instance(&entry, &window)?;

        // Extensions: Debug & Surface
        #[cfg(not(feature = "render_dbg"))]
        let (debug_ext_loader, debug_ext_messenger) = (None, None);

        #[cfg(feature = "render_dbg")]
        let (debug_ext_loader, debug_ext_messenger) = {
            let debug_ext = DebugExtension::new(&entry, &instance)?;
            (Some(debug_ext.loader), Some(debug_ext.messenger))
        };

        let surface_ext = SurfaceExtension::new(&entry, &instance, &window)?;

        // Device
        let device = Device::new(&instance, &surface_ext)?;

        // Queue Families
        let graphics_queue = unsafe {
            device
                .logical_device
                .get_device_queue(device.graphics_queue_index, 0)
        };

        let present_queue = unsafe {
            device
                .logical_device
                .get_device_queue(device.present_queue_index, 0)
        };

        // Extension: Swapchain
        let mut swapchain_ext = SwapchainExtension::new(
            &entry,
            &instance,
            &device.logical_device,
            &device.physical_device,
            &surface_ext,
            &window,
        )?;

        let swapchain_images = unsafe {
            swapchain_ext
                .loader
                .get_swapchain_images(swapchain_ext.swapchain)
        }?;

        // Image views
        let mut image_views: Vec<vk::ImageView> = {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1)
                .build();

            let mut image_views = Vec::new();
            for img in swapchain_images {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_SRGB)
                    .subresource_range(subresource_range);

                image_views
                    .push(unsafe { device.logical_device.create_image_view(&create_info, None) }?);
            }
            image_views
        };

        // Descriptor
        let descriptor = Descriptor::new(&device.logical_device, Self::MAX_FRAMES_INFLIGHT)?;

        // Push Constants
        let push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .size(std::mem::size_of::<glm::Mat4>() as u32)
            .offset(0)
            .build();

        // Viewport & Scissor
        let mut viewport = vk::Viewport {
            width: window.inner_size().width as f32,
            height: window.inner_size().height as f32,
            max_depth: 1.0,
            ..Default::default()
        };

        let mut scissor = vk::Rect2D {
            extent: vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            },
            ..Default::default()
        };

        // Graphics Pipeline
        let graphics_pipeline = GraphicsPipeline::new(
            &device.logical_device,
            &descriptor.set_layout,
            &viewport,
            &scissor,
            std::mem::size_of::<Vertex>() as u32,
            &push_constant_range,
        )?;

        // Buffers
        let draw_command_buffer = buffers::CommandBuffer::new_draw_cmd_buffer(
            &device.logical_device,
            device.graphics_queue_index,
            Self::MAX_FRAMES_INFLIGHT as u32,
        )?;

        let mut frame_buffer = buffers::FrameBuffer::new(
            &device.logical_device,
            &image_views,
            &graphics_pipeline.render_pass,
            window_size.width,
            window_size.height,
        )?;

        let vertices_size = (std::mem::size_of::<Vertex>() * object_pool.vertices.len()) as u64;

        let vertex_buffer = buffers::StorageBuffer::new(
            &device.logical_device,
            &device.memory_properties,
            &graphics_queue,
            &device.graphics_queue_index,
            vertices_size,
            DataUsage::VERTEX,
            &object_pool.vertices,
            std::mem::align_of::<f32>() as u64,
        )?;

        let indices_size = (std::mem::size_of::<u16>() * object_pool.indices.len()) as u64;

        let index_buffer = buffers::StorageBuffer::new(
            &device.logical_device,
            &device.memory_properties,
            &graphics_queue,
            &device.graphics_queue_index,
            indices_size,
            DataUsage::INDEX,
            &object_pool.indices,
            (std::mem::align_of::<u16>()) as u64,
        )?;

        let uniform_buffer = buffers::UniformBuffer::new(
            &device.logical_device,
            &device.memory_properties,
            Self::MAX_FRAMES_INFLIGHT,
            (std::mem::size_of::<CameraVP>()) as u64,
        )?;

        descriptor.update_descriptor_sets(
            &device.logical_device,
            Self::MAX_FRAMES_INFLIGHT,
            &uniform_buffer.buffers,
            std::mem::size_of::<CameraVP>() as u64,
        )?;

        // Syncronization
        let mut semaphores_release: Vec<vk::Semaphore> =
            Vec::with_capacity(Self::MAX_FRAMES_INFLIGHT);

        let mut semaphores_acquire: Vec<vk::Semaphore> =
            Vec::with_capacity(Self::MAX_FRAMES_INFLIGHT);

        let mut fences_inflight: Vec<vk::Fence> = Vec::with_capacity(Self::MAX_FRAMES_INFLIGHT);

        for _ in 0..Self::MAX_FRAMES_INFLIGHT {
            semaphores_release.push(unsafe {
                device
                    .logical_device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?);

            semaphores_acquire.push(unsafe {
                device
                    .logical_device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }?);

            fences_inflight.push(unsafe {
                device.logical_device.create_fence(
                    &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
            }?);
        }

        // Render Loop Data
        let scene = Scene::new(&window, ProjectionType::Orthographic);
        let draw_pool = Vec::new(); // <- Empty Draw Pool
        let render_stats = RenderStats::new();

        Ok(Self {
            // Base
            entry,
            instance,
            device: device.logical_device,
            physical_device: device.physical_device,
            image_views,

            // Extensions
            debug_utils_loader: debug_ext_loader,
            debug_messenger: debug_ext_messenger,
            surface_loader: surface_ext.loader,
            surface: surface_ext.surface,
            swapchain_loader: swapchain_ext.loader,
            swapchain: swapchain_ext.swapchain,

            // Descriptors
            descriptor_set_layout: descriptor.set_layout,
            descriptor_pool: descriptor.pool,
            descriptor_sets: descriptor.sets,

            // Graphics Pipeline
            pipeline_layout: graphics_pipeline.layout,
            render_pass: graphics_pipeline.render_pass,
            graphics_pipeline: graphics_pipeline.pipeline,
            graphics_queue,
            present_queue,
            viewport,
            scissor,
            push_constant_range,

            // Buffers
            frame_buffers: frame_buffer.buffers,
            command_pool: draw_command_buffer.pool,
            draw_command_buffers: draw_command_buffer.buffers,
            vertex_buffer: vertex_buffer.buffer,
            vertex_buffer_memory: vertex_buffer.buffer_memory,
            index_buffer: index_buffer.buffer,
            index_buffer_memory: index_buffer.buffer_memory,
            uniform_buffers: uniform_buffer.buffers,
            uniform_buffers_memory: uniform_buffer.buffers_memory,
            uniform_buffers_mapped: uniform_buffer.buffers_mapped,
            uniform_buffers_mem_req: uniform_buffer.buffers_mem_req,

            // Syncronization
            semaphores_acquire,
            semaphores_release,
            fences_inflight,

            // Render Loop Data
            current_frame: 0,
            scene,
            object_pool,
            draw_pool,
            render_stats,
        })
    }

    /* Swapchain */

    /// Recreates the [`Swapchain`] based on the `new_size`
    ///
    /// Recration occurs only when `new_size` is valid
    pub fn recreate_swapchain(&mut self, new_size: PhysicalSize<u32>) -> Result<()> {
        // Window Minimized -> No Recreation
        if new_size.height == 0 || new_size.width == 0 {
            return Ok(());
        }

        // Cleanup Old Swapchain
        unsafe {
            self.device.device_wait_idle();
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            for iv in &self.image_views {
                self.device.destroy_image_view(*iv, None);
            }

            for fb in &self.frame_buffers {
                self.device.destroy_framebuffer(*fb, None)
            }
        }

        // Adjust Dynamic State
        self.viewport.height = new_size.height as f32;
        self.viewport.width = new_size.width as f32;

        self.scissor.extent.height = new_size.height;
        self.scissor.extent.width = new_size.width;

        // Recreate Swapchain / ImageViews / FrameBuffers
        self.swapchain = {
            let (min_image_count, pre_transform) = {
                let caps = unsafe {
                    self.surface_loader
                        .get_physical_device_surface_capabilities(
                            self.physical_device,
                            self.surface,
                        )
                }?;
                let mut count = caps.min_image_count + 1;

                if caps.max_image_count > 0 && count > caps.max_image_count {
                    count = caps.max_image_count;
                }

                (count, caps.current_transform)
            };

            // TODO! -> This is too strict/error prone right now, better to supplement with queried data
            // TODO! -> Check for defaults
            let create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(self.surface)
                .min_image_count(min_image_count)
                .image_format(vk::Format::B8G8R8A8_SRGB)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(self.scissor.extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(vk::PresentModeKHR::MAILBOX)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .clipped(true);

            // TODO! -> Fix me!
            // let create_info = if !queue_family_indices.eq_indicies()? {
            //     create_info
            //         .image_sharing_mode(vk::SharingMode::CONCURRENT)
            //         .queue_family_indices(queue_family_indices.to_vec()?)
            //     //.queue_family_indices(&queue_family_indices.to_vec()?.as_slice());
            // } else {
            //     create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            // };

            unsafe { self.swapchain_loader.create_swapchain(&create_info, None) }?
            // TODO! -> STATUS_STACK_BUFFER_OVERRUN Error
        };

        let swapchain_images =
            unsafe { self.swapchain_loader.get_swapchain_images(self.swapchain) }?;

        self.image_views = {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1)
                .build();

            let mut image_views = Vec::new();
            for img in swapchain_images {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_SRGB)
                    .subresource_range(subresource_range);

                image_views.push(unsafe { self.device.create_image_view(&create_info, None) }?);
            }
            image_views
        };

        self.frame_buffers = buffers::FrameBuffer::new(
            &self.device,
            &self.image_views,
            &self.render_pass,
            new_size.width,
            new_size.height,
        )?
        .buffers;

        Ok(())
    }

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

        /////////////////// STATISTICS TEXT ///////////////////
        self.text(
            &self.render_stats.as_text(),
            1.0,
            -1.5,
            0.75,
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
            );

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

            let render_pass_begin = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(
                    *self
                        .frame_buffers
                        .get(image_index as usize)
                        .context("Frame Buffer: Index out of bounds")?,
                )
                .render_area(self.scissor)
                .clear_values(&Self::CLEAR_VALUES);

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

            let set = self
                .descriptor_sets
                .get(self.current_frame)
                .context("Descriptor Sets: Index out of bounds")?;

            self.device.cmd_bind_descriptor_sets(
                self.draw_command_buffers[self.current_frame],
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                std::slice::from_ref(set),
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

            self.scene.update_projection(&window);

            let mut uniform_align = util::Align::new(
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

            uniform_align.copy_from_slice(&std::slice::from_ref(&self.scene.camera_vp));

            let submit_info = vk::SubmitInfo::builder()
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

            let present_info = vk::PresentInfoKHR::builder()
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
        let mut object_transform: glm::Mat4;

        for draw_instance in &self.draw_pool {
            object_transform = glm::translate(
                &glm::Mat4::identity(),
                &draw_instance.position, // Object Position
            ) * glm::rotate(
                &glm::Mat4::identity(),
                (draw_instance.rotation).to_radians(), // Rotation
                &glm::vec3(0.0, 0.0, 1.0),             // Axis of Rotation
            ) * glm::scale(
                &glm::Mat4::identity(),
                &glm::Vec3::from_element(draw_instance.scale), // Scale Factors
            );

            unsafe {
                self.device.cmd_push_constants(
                    self.draw_command_buffers[self.current_frame],
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    &bytemuck::try_cast_slice(object_transform.as_slice())?,
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

    /// Creates and pushes a circle object to draw
    pub fn circle(
        &mut self,
        scale: f32,
        center_x: f32,
        center_y: f32,
        anchor_type: AnchorType,
    ) -> Result<()> {
        let anchor_position = match anchor_type {
            AnchorType::Locked => glm::vec3(
                center_x + self.scene.camera_pos.x,
                center_y + self.scene.camera_pos.y,
                0.0,
            ),
            AnchorType::Unlocked => glm::vec3(center_x, center_y, 0.0),
        };

        self.draw_pool.push(ObjectInstance {
            position: anchor_position,
            rotation: 0.0,
            scale,
            object_index: self.object_pool.pool.len() - 1,
        });

        Ok(())
    }

    /// Creates and pushes a rectangle object to draw
    pub fn rectangle(
        &mut self,
        scale: f32,
        center_x: f32,
        center_y: f32,
        anchor_type: AnchorType,
    ) -> Result<()> {
        let anchor_position = match anchor_type {
            AnchorType::Locked => glm::vec3(
                center_x + self.scene.camera_pos.x,
                center_y + self.scene.camera_pos.y,
                0.0,
            ),
            AnchorType::Unlocked => glm::vec3(center_x, center_y, 0.0),
        };

        self.draw_pool.push(ObjectInstance {
            position: anchor_position,
            rotation: 0.0,
            scale,
            object_index: self.object_pool.pool.len() - 2,
        });

        Ok(())
    }

    /// Creates and pushes a text object to draw
    pub fn text(
        &mut self,
        text: &str,
        scale: f32,
        top_left_x: f32,
        top_left_y: f32,
        anchor_type: AnchorType,
    ) -> Result<()> {
        // let scale = scale * self.scene.camera_zoom;
        let pad_x = scale * 0.03;
        let pad_y = scale * 0.05;

        let anchor_position = match anchor_type {
            AnchorType::Locked => glm::vec3(
                top_left_x + self.scene.camera_pos.x + pad_x,
                top_left_y + self.scene.camera_pos.y - pad_y,
                0.0,
            ),
            AnchorType::Unlocked => glm::vec3(top_left_x + pad_x, top_left_y - pad_y, 0.0),
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
                text_instance_pool.push(ObjectInstance {
                    position: cursor_position,
                    scale: scale,
                    object_index: char_index as usize,
                    ..ObjectInstance::default()
                });
            }

            // Move the cursor by 1 character to right
            cursor_position.x += pad_x;
        }

        self.draw_pool.extend(text_instance_pool);

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

            self.render_stats.changed = true;
        } else {
            self.render_stats.frame_counter += 1;
        }

        // Update Pool Stats
        if self.render_stats.last_draw_pool_elements != self.draw_pool.len() {
            self.render_stats.last_draw_pool_elements = self.draw_pool.len();
            self.render_stats.changed = true;
        }

        if self.render_stats.last_draw_pool_vertices != self.object_pool.vertices.len() {
            self.render_stats.last_draw_pool_vertices = self.object_pool.vertices.len();
            self.render_stats.changed = true;
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle();

            // Buffers: Index & Vertex
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);

            // Syncronisation
            self.semaphores_acquire.clone().into_iter().for_each(|s| {
                self.device.destroy_semaphore(s, None);
            });
            self.semaphores_release.clone().into_iter().for_each(|s| {
                self.device.destroy_semaphore(s, None);
            });
            self.fences_inflight.clone().into_iter().for_each(|f| {
                self.device.destroy_fence(f, None);
            });

            // Command Pool
            self.device.destroy_command_pool(self.command_pool, None);

            // Buffers: Frame & Uniform
            self.frame_buffers
                .clone()
                .into_iter()
                .for_each(|fb| self.device.destroy_framebuffer(fb, None));
            self.uniform_buffers
                .clone()
                .into_iter()
                .for_each(|b| self.device.destroy_buffer(b, None));
            self.uniform_buffers_memory
                .clone()
                .into_iter()
                .for_each(|dm| self.device.free_memory(dm, None));

            // Descriptors & Pipeline
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.render_pass, None);
            self.image_views
                .clone() // TODO! -> Potential fix here, but cloning Handles should be OK
                .into_iter()
                .for_each(|iv| self.device.destroy_image_view(iv, None));

            // Extensions: Swapchain & Surface
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);

            // Device
            self.device.destroy_device(None);

            // Extension: Debug
            if let (Some(debug_utils_loader), Some(debug_messenger)) =
                (&self.debug_utils_loader, self.debug_messenger)
            {
                debug_utils_loader.destroy_debug_utils_messenger(debug_messenger, None);
            };

            // Instance
            self.instance.destroy_instance(None);
        }
    }
}

/// Cretes a Vulkan Instance using the given `entry` and `window`
pub fn create_instance(
    entry: &ash::Entry,
    window: &winit::window::Window,
) -> Result<ash::Instance> {
    /* Application Data */
    let api_version = match entry.try_enumerate_instance_version()? {
        Some(v) if vk::api_version_minor(v) >= 3 => Ok(vk::API_VERSION_1_3),
        _ => Err(anyhow!("Atleast Vulkan Version 1.3 needed")),
    }?;

    let application_info = vk::ApplicationInfo::builder()
        .application_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"lavapond\0") })
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"vulkan\0") })
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(api_version);

    /* Extensions */
    let mut enabled_extension_names =
        ash_window::enumerate_required_extensions(window.raw_display_handle())?.to_vec();

    enabled_extension_names.push(khr::Surface::name().as_ptr());

    #[cfg(feature = "render_dbg")]
    enabled_extension_names.push(ext::DebugUtils::name().as_ptr());

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_extension_names(&enabled_extension_names);

    /* Layers */
    #[cfg(feature = "render_dbg")]
    let enabled_layer_names = vec![unsafe {
        CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()
    }];

    /* Validation Features*/
    #[allow(unused_mut)]
    #[cfg(feature = "render_dbg")]
    let mut enabled_validation_features = vec![];

    #[cfg(all(feature = "render_dbg", feature = "best_practices"))]
    enabled_validation_features.push(vk::ValidationFeatureEnableEXT::BEST_PRACTICES);

    #[cfg(all(feature = "render_dbg", feature = "debug_printf"))]
    enabled_validation_features.push(vk::ValidationFeatureEnableEXT::DEBUG_PRINTF);

    #[cfg(all(feature = "render_dbg", feature = "gpu_assist"))]
    enabled_validation_features.push(vk::ValidationFeatureEnableEXT::GPU_ASSISTED);

    #[cfg(all(feature = "render_dbg", feature = "sync_validation"))]
    enabled_validation_features.push(vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION);

    #[cfg(feature = "render_dbg")]
    let mut validation_features = vk::ValidationFeaturesEXT::builder()
        .enabled_validation_features(&enabled_validation_features);

    #[cfg(feature = "render_dbg")]
    let create_info = create_info
        .enabled_layer_names(&enabled_layer_names)
        .push_next(&mut validation_features);

    Ok(unsafe { entry.create_instance(&create_info, None) }?)
}

struct Device {
    physical_device: vk::PhysicalDevice,
    logical_device: ash::Device,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    graphics_queue_index: u32,
    present_queue_index: u32,
    // transfer_queue_index: u32,
}

impl Device {
    // TODO! -> This is too strict right now, better to rank surface properties
    // TODO! -> Capability Support: image count + image extent

    /// Creates a new device using the given `instance` and `surface_ext
    fn new(instance: &ash::Instance, surface_ext: &SurfaceExtension) -> Result<Self> {
        /*Find Physical Device*/
        let mut physical_device = None;
        let mut graphics_queue_index = None;
        let mut present_queue_index = None;
        // let mut transfer_queue_index = None;

        for pd in unsafe { instance.enumerate_physical_devices() }? {
            /* Device Properties */
            if !(unsafe { instance.get_physical_device_properties(pd) }.device_type
                == vk::PhysicalDeviceType::DISCRETE_GPU)
            {
                continue;
            }

            /* Device Features */
            // unsafe { instance.get_physical_device_features(*pd) }

            /* Extension Properties */
            if !(unsafe { instance.enumerate_device_extension_properties(pd) }?
                .into_iter()
                .any(|ep| unsafe { CStr::from_ptr(ep.extension_name.as_ptr()) } == khr::Swapchain::name())) {
					continue;
				}

            /* Surface Capability */
            // unsafe { surface.get_physical_device_surface_capabilities(*pd, surface_khr) }?

            /* Surface Formats */
            if !(unsafe {
                surface_ext
                    .loader
                    .get_physical_device_surface_formats(pd, surface_ext.surface)
            }?
            .into_iter()
            .any(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })) {
                continue;
            }

            /* Surface Present Modes */
            if !(unsafe {
                surface_ext
                    .loader
                    .get_physical_device_surface_present_modes(pd, surface_ext.surface)
            }?
            .into_iter()
            .any(|pm| pm == vk::PresentModeKHR::MAILBOX))
            {
                continue;
            }

            /* Queue Family Indices */
            graphics_queue_index = None;
            present_queue_index = None;
            if !(unsafe { instance.get_physical_device_queue_family_properties(pd) }
                .into_iter()
                .enumerate()
                .any(|(i, qf)| {
                    let index = i as u32;

                    if graphics_queue_index.is_none()
                        && qf.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    {
                        graphics_queue_index = Some(index);
                    }

                    if present_queue_index.is_none()
                        && unsafe {
                            surface_ext.loader.get_physical_device_surface_support(
                                pd,
                                index,
                                surface_ext.surface,
                            )
                        }
                        .unwrap_or(false)
                    {
                        // Present Index must be different than Graphics Index
                        if let Some(graphics_queue_index) = graphics_queue_index {
                            if graphics_queue_index != index {
                                present_queue_index = Some(index);
                            }
                        }
                    }

                    // if transfer_queue_index.is_none()
                    //     && qf.queue_flags.contains(vk::QueueFlags::TRANSFER)
                    // {
                    //     // Compute Index must be different than Graphics & Present Index
                    //     if let (Some(graphics_queue_index), Some(present_queue_index)) =
                    //         (graphics_queue_index, present_queue_index)
                    //     {
                    //         if graphics_queue_index != index && present_queue_index != index {
                    //             transfer_queue_index = Some(index);
                    //         }
                    //     }
                    // }

                    graphics_queue_index.is_some() && present_queue_index.is_some()
                    // && transfer_queue_index.is_some()
                }))
            {
                continue;
            }

            physical_device = Some(pd);
            break;
        }

        let physical_device =
            physical_device.context("Could not find a proper physical device!")?;
        let graphics_queue_index = graphics_queue_index.unwrap();
        let present_queue_index = present_queue_index.unwrap();
        // let transfer_queue_index = transfer_queue_index.unwrap();

        /* Physical Device Memory Properties */
        let memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        /* Create Logical Device */
        let logical_device = {
            let queue_priority = [1.0];

            let queue_create_infos = vec![
                // Graphics Queue
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(graphics_queue_index)
                    .queue_priorities(&queue_priority)
                    .build(),
                // Present Queue
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(present_queue_index)
                    .queue_priorities(&queue_priority)
                    .build(),
                // Transfer Queue
                // vk::DeviceQueueCreateInfo::builder()
                //     .queue_family_index(transfer_queue_index)
                //     .queue_priorities(&queue_priority)
                //     .build(),
            ];

            let extension_names = [khr::Swapchain::name().as_ptr()];

            let create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&extension_names);

            unsafe { instance.create_device(physical_device, &create_info, None) }?
        };

        Ok(Self {
            physical_device,
            logical_device,
            memory_properties,
            graphics_queue_index,
            present_queue_index,
            // transfer_queue_index,
        })
    }
}

//==================================================
//=== Render Statistics
//==================================================

struct RenderStats {
    turned_off: bool,
    frames_per_sec: u32,
    last_draw_request_time: u128,
    last_draw_pool_creation_time: u128,
    last_draw_pool_elements: usize,
    last_draw_pool_vertices: usize,
    changed: bool,
    frame_counter: u32,
    fps_instant: Instant,
    draw_request_instant: Instant,
    pool_creation_instant: Instant,
}

impl RenderStats {
    /// Creates a new render statistics
    fn new() -> Self {
        Self {
            turned_off: true, // false
            frames_per_sec: 0,
            last_draw_request_time: 0,
            last_draw_pool_creation_time: 0,
            last_draw_pool_elements: 0,
            last_draw_pool_vertices: 0,
            changed: false,
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
        self.changed = true;
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
        self.changed = true;
    }

    /// Gives back the current stats as a [`String`]
    fn as_text(&self) -> String {
        format!("[Statistics]\nfps: {}\nrequest time: {} us\npool creation time:{}\nelements:{}\nvertices:{}", 
        self.frames_per_sec,
        self.last_draw_request_time,
        self.last_draw_pool_creation_time,
        self.last_draw_pool_elements,
        self.last_draw_pool_vertices)
    }
}

//==================================================
//=== Draw Instance
//==================================================

pub enum AnchorType {
    Locked,
    Unlocked,
}

//==================================================
//=== Render Loop
//==================================================

pub struct Scene {
    camera_zoom: f32,
    camera_pos: glm::Vec3,
    camera_vp: CameraVP,
    projection: ProjectionType,
}

impl Scene {
    /// Creates a new [`Scene`] based on the current windows size
    pub fn new(window: &winit::window::Window, projection_type: ProjectionType) -> Self {
        let aspect = (window.inner_size().width / window.inner_size().height) as f32;
        let camera_pos = glm::vec3(0.0, 0.0, 2.0);
        let camera_vp = CameraVP::new(&camera_pos, &projection_type, aspect);

        Self {
            camera_zoom: 1.0,
            camera_pos,
            camera_vp,
            projection: projection_type,
        }
    }

    /// Change the current zoom level with the value of `delta`
    pub fn zoom(&mut self, delta: f32) -> () {
        self.camera_zoom = f32::clamp(self.camera_zoom + delta, 0.1, 2.0);
    }

    /// Pan the came on the X and Y axis
    pub fn pan_view_xy(&mut self, x: f32, y: f32) -> () {
        self.camera_pos = glm::vec3(
            self.camera_pos.x + x,
            self.camera_pos.y - y,
            self.camera_pos.z,
        );

        self.camera_vp.view = glm::look_at(
            &self.camera_pos,                                      // Camera Position
            &glm::vec3(self.camera_pos.x, self.camera_pos.y, 0.0), // Camera Target
            &glm::vec3(0.0, 1.0, 0.0),
        );
    }

    /// Updates the projection matrix of the camera
    ///
    /// If the camera is fix then we do not need to call this function
    pub fn update_projection(&mut self, window: &winit::window::Window) -> () {
        //let n = 2.0 * self.camera_zoom;

        let target_width = 4.0;
        let target_height = 3.0;
        let target_aspect = target_width / target_height;
        let viewport_aspect =
            (window.inner_size().width as f32) / (window.inner_size().height as f32);

        match self.projection {
            ProjectionType::Orthographic => {
                if target_aspect >= viewport_aspect {
                    self.camera_vp.projection = glm::ortho(
                        -viewport_aspect / target_aspect * target_width / 2.0, // -n * 0.5,
                        viewport_aspect / target_aspect * target_width / 2.0,  // n * 0.5,
                        -target_height / 2.0,                                  // -n * 0.5 / aspect,
                        target_height / 2.0,                                   // n * 0.5 / aspect,
                        -100.0,
                        100.0,
                    );
                } else {
                    self.camera_vp.projection = glm::ortho(
                        -target_width / 2.0,                                    // -n * 0.5,
                        target_width / 2.0,                                     // n * 0.5,
                        -target_aspect / viewport_aspect * target_height / 2.0, // -n * 0.5 / aspect,
                        target_aspect / viewport_aspect * target_height / 2.0,  // n * 0.5 / aspect,
                        -100.0,
                        100.0,
                    );
                }
            }
            ProjectionType::Perspective => {
                self.camera_vp.projection =
                    glm::perspective(viewport_aspect, (60.0f32).to_radians(), 0.1, 20.0);
            }
        };

        self.camera_vp.projection[(1, 1)] *= -1.0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CameraVP {
    view: glm::Mat4,
    projection: glm::Mat4,
}

impl CameraVP {
    /// Creates a new [`CameraVP`]
    pub fn new(position: &glm::Vec3, projection_type: &ProjectionType, aspect: f32) -> Self {
        let mut projection = match projection_type {
            ProjectionType::Orthographic => {
                glm::ortho(-1.0, 1.0, -1.0 / aspect, 1.0 / aspect, -100.0, 100.0)
            }

            ProjectionType::Perspective => {
                glm::perspective(aspect, (60.0f32).to_radians(), 0.1, 10.0)
            }
        };
        projection[(1, 1)] *= -1.0;

        Self {
            view: glm::look_at(
                position,                                // Camera Position
                &glm::vec3(position.x, position.y, 0.0), // Camera Target
                &glm::vec3(0.0, 1.0, 0.0),               // Up Axis
            ),
            projection,
        }
    }
}

pub enum ProjectionType {
    Orthographic,
    Perspective,
}
