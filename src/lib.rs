mod buffers;
mod descriptor;
mod extensions;
mod pipeline;
mod resources;

pub mod camera;
pub mod coord_sys;
pub mod shapes;

use crate::buffers::*;
use crate::camera::*;
use crate::coord_sys::*;
use crate::descriptor::*;
use crate::extensions::*;
use crate::pipeline::*;
use crate::resources::*;
use crate::shapes::*;
use anyhow::{Context, Ok, Result, anyhow};
use ash::{
    ext, khr,
    vk::{self, DescriptorSet},
};
use glam;
use raw_window_handle::HasDisplayHandle;
use std::{
    ffi::CStr,
    time::{Duration, Instant},
};
use utils::color::Color;
use winit::dpi::PhysicalSize;

//==================================================
//=== Renderer
//==================================================

pub struct Renderer {
    // Vulkan: Base
    #[allow(dead_code)]
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    image_views: Vec<vk::ImageView>,

    // Vulkan: Extensions
    debug_utils_loader: Option<ext::debug_utils::Instance>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    surface_loader: khr::surface::Instance,
    surface: vk::SurfaceKHR,
    swapchain_loader: khr::swapchain::Device,
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
    clear_color: [vk::ClearValue; 1],
    viewport: vk::Viewport,
    scissor: vk::Rect2D,
    #[allow(dead_code)]
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
    pub camera: camera::Camera,
    object_pool: ObjectPool,
    pub draw_pool: Vec<ObjectInstance>,
    render_stats: RenderStats,
}

impl Renderer {
    const MAX_FRAMES_INFLIGHT: usize = 2;

    /// Creates a new [`Renderer`] using `window`
    pub fn new(window: &winit::window::Window, clear_color: Color) -> Result<Renderer> {
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
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);

            let mut image_views = Vec::new();
            for img in swapchain_images {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(img)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(vk::Format::B8G8R8A8_SRGB)
                    .subresource_range(subresource_range);

                image_views
                    .push(unsafe { device.logical_device.create_image_view(&create_info, None) }?);
            }
            image_views
        };

        // Clear Color
        let clear_color = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [clear_color.r(), clear_color.g(), clear_color.b(), 1.0],
            },
        }];

        // Descriptor
        let descriptor = Descriptor::new(&device.logical_device, Self::MAX_FRAMES_INFLIGHT)?;

        // Push Constants
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .size(std::mem::size_of::<DrawInstanceData>() as u32)
            .offset(0);

        // Viewport & Scissor
        let viewport = vk::Viewport {
            width: window.inner_size().width as f32,
            height: window.inner_size().height as f32,
            max_depth: 1.0,
            ..Default::default()
        };

        let scissor = vk::Rect2D {
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

        let frame_buffer = buffers::FrameBuffer::new(
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
            (std::mem::size_of::<ViewProjection>()) as u64,
        )?;

        descriptor.update_descriptor_sets(
            &device.logical_device,
            Self::MAX_FRAMES_INFLIGHT,
            &uniform_buffer.buffers,
            std::mem::size_of::<ViewProjection>() as u64,
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
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )
            }?);
        }

        // Camera

        let camera = Camera::new(
            &window,
            glam::vec3(0.0, 0.0, 2.0),
            ProjectionType::Orthographic,
        );

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
            clear_color,
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
            camera,
            object_pool,
            draw_pool: Vec::new(),
            render_stats: RenderStats::new(),
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
            self.device.device_wait_idle()?;
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
            let create_info = vk::SwapchainCreateInfoKHR::default()
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
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1);

            let mut image_views = Vec::new();
            for img in swapchain_images {
                let create_info = vk::ImageViewCreateInfo::default()
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
        let mut draw_instance_data = DrawInstanceData::new_empty();

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

    /// https://registry.khronos.org/vulkan/specs/latest/man/html/vkDeviceWaitIdle.html
    ///
    /// Should be called before dropping renderer
    pub fn wait_device_idle(&self) -> Result<()> {
        unsafe { self.device.device_wait_idle()? }

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
                text_instance_pool.push(ObjectInstance {
                    position: cursor_position,
                    scale: glam::vec3(scale, scale, 0.0),
                    object_index: char_index as usize,
                    color: Color::EMERALD,
                    ..ObjectInstance::default()
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
            ShapeType::Circle => ObjectPool::CIRCLE,
            ShapeType::CircleBorder => ObjectPool::CIRCLE_BORDER,
            ShapeType::Rectangle => ObjectPool::RECTANGLE,
            ShapeType::RectangleBorder => ObjectPool::RECTANGLE_BORDER,
            ShapeType::RoundedRectangle => ObjectPool::ROUNDED_RECTANGLE,
            ShapeType::RoundedRectangleBorder => ObjectPool::ROUNDED_RECTANGLE_BORDER,
        };

        self.draw_pool.push(ObjectInstance {
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

    /* Misc */

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

    pub fn screen_to_world(&self, position: ScreenPos2D) -> WorldPos2D {
        let half_width = self.camera.get_width() * 0.5;
        let half_height = self.camera.get_height() * 0.5;
        let x = (position.x - half_width) / half_width;
        let y = (position.y - half_height) / half_height;

        WorldPos2D::new(x, y)
    }

    pub fn world_to_screen(&self, position: WorldPos2D) -> ScreenPos2D {
        let half_width = self.camera.get_width() * 0.5;
        let half_height = self.camera.get_height() * 0.5;
        let x = (position.x * half_width) + half_width;
        let y = (position.y * half_height) + half_height;

        ScreenPos2D::new(x, y)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
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
    let api_version = match unsafe { entry.try_enumerate_instance_version()? } {
        Some(v) if vk::api_version_minor(v) >= 3 => Ok(vk::API_VERSION_1_3),
        _ => Err(anyhow!("Atleast Vulkan Version 1.3 needed")),
    }?;

    let application_info = vk::ApplicationInfo::default()
        .application_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"lavapond\0") })
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"lavapond\0") })
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(api_version);

    /* Extensions */
    let mut enabled_extension_names =
        ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?.to_vec();

    enabled_extension_names.push(khr::surface::NAME.as_ptr());

    #[cfg(feature = "render_dbg")]
    enabled_extension_names.push(ext::debug_utils::NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
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
    let mut validation_features = vk::ValidationFeaturesEXT::default()
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
    const EXTENSION_NAMES: [*const i8; 1] = [khr::swapchain::NAME.as_ptr()];

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
            let mut extensions_found = 0;
            for ep in unsafe { instance.enumerate_device_extension_properties(pd) }? {
                let extension_name = unsafe { CStr::from_ptr(ep.extension_name.as_ptr()) };
                for required_name in Device::EXTENSION_NAMES {
                    if extension_name == unsafe { CStr::from_ptr(required_name) } {
                        extensions_found += 1;
                    }
                }
            }

            if extensions_found < Device::EXTENSION_NAMES.len() {
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
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(graphics_queue_index)
                    .queue_priorities(&queue_priority),
                // Present Queue
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(present_queue_index)
                    .queue_priorities(&queue_priority),
                // Transfer Queue
                // vk::DeviceQueueCreateInfo::default()
                //     .queue_family_index(transfer_queue_index)
                //     .queue_priorities(&queue_priority),
            ];

            let create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&Device::EXTENSION_NAMES);

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
    frame_counter: u32,
    fps_instant: Instant,
    draw_request_instant: Instant,
    pool_creation_instant: Instant,
}

impl RenderStats {
    /// Creates a new render statistics
    fn new() -> Self {
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

pub struct DrawInstanceData {
    transform: glam::Mat4,
    color: Color,
}

impl DrawInstanceData {
    const TRANSFORM_LEN: usize = 4 * 4;
    const COLOR_LEN: usize = 3;

    /// Creates a new empty [`DrawInstanceData`]
    pub fn new_empty() -> Self {
        Self {
            transform: glam::Mat4::ZERO,
            color: Color::BLACK,
        }
    }

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
