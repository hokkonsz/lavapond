// extern
use anyhow::{anyhow, Result};
use ash::{util, vk};

//==================================================
//=== Commad Buffer
//==================================================

pub struct CommandBuffer {
    pub pool: vk::CommandPool,
    pub buffers: Vec<vk::CommandBuffer>,
}

impl CommandBuffer {
    /// Creates a new (Draw) [`CommandBuffer`]
    pub fn new_draw_cmd_buffer(
        logical_device: &ash::Device,
        queue_family_index: u32,
        buffer_count: u32,
    ) -> Result<Self> {
        let pool = {
            let create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family_index);

            unsafe { logical_device.create_command_pool(&create_info, None) }?
        };

        let buffers = {
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(buffer_count);

            unsafe { logical_device.allocate_command_buffers(&allocate_info) }?
        };

        Ok(Self { pool, buffers })
    }

    /// Copy the data of a buffer into another one
    ///
    /// Using:
    /// * Transient Command Pool (Buffers with short lifetime)
    /// * Onetime Submit Command Buffers
    pub fn buffer_copy(
        logical_device: &ash::Device,
        queue: &vk::Queue,
        queue_family_index: &u32,
        data_sizes: &[u64],
        src_buffers: &[&vk::Buffer],
        dst_buffers: &[&vk::Buffer],
    ) -> Result<()> {
        if data_sizes.len() != src_buffers.len() || data_sizes.len() != dst_buffers.len() {
            return Err(anyhow!("Length of input vectors must match!"));
        }

        let pool = {
            let create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(*queue_family_index);

            unsafe { logical_device.create_command_pool(&create_info, None) }?
        };

        let buffers = {
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            unsafe { logical_device.allocate_command_buffers(&allocate_info) }?
        };

        unsafe {
            /* Start Recording */
            logical_device.begin_command_buffer(
                buffers[0],
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            /* Commands */
            for i in 0..data_sizes.len() {
                logical_device.cmd_copy_buffer(
                    buffers[0],
                    *src_buffers[i],
                    *dst_buffers[i],
                    &[vk::BufferCopy::default().size(data_sizes[i])],
                );
            }

            /* End Recording */
            logical_device.end_command_buffer(buffers[0])?;

            /* Submit To Queue */
            let submit_info = vk::SubmitInfo::default().command_buffers(&buffers);

            logical_device.queue_submit(
                *queue,
                std::slice::from_ref(&submit_info),
                vk::Fence::null(),
            )?;

            /* Cleanup*/
            logical_device.queue_wait_idle(*queue)?;
            logical_device.destroy_command_pool(pool, None);
        }

        Ok(())
    }
}

//==================================================
//=== Frame Buffer
//==================================================

pub struct FrameBuffer {
    pub buffers: Vec<vk::Framebuffer>,
}

impl FrameBuffer {
    /// Creates a new [`FrameBuffer`]
    pub fn new(
        logical_device: &ash::Device,
        image_views: &Vec<vk::ImageView>,
        render_pass: &vk::RenderPass,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let mut buffers = Vec::new();

        for iv in image_views {
            let iv = [*iv];

            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(*render_pass)
                .attachments(&iv)
                .width(width)
                .height(height)
                .layers(1);

            buffers.push(unsafe { logical_device.create_framebuffer(&create_info, None) }?);
        }

        Ok(Self { buffers })
    }
}

//==================================================
//=== Storage Buffer
//==================================================

pub enum DataUsage {
    VERTEX,
    INDEX,
}

pub struct StorageBuffer {
    pub buffer: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
}

impl StorageBuffer {
    /// Creates a new [`StorageBuffer`]
    ///
    /// Buffer Creation Steps:
    /// 1. Stage data using staging buffer
    /// 2. Create storage buffer
    /// 3. Copy data from staging buffer to storage buffer
    pub fn new<T: Copy>(
        logical_device: &ash::Device,
        device_mem_properties: &vk::PhysicalDeviceMemoryProperties,
        queue: &vk::Queue,
        queue_family_index: &u32,
        data_size: u64,
        data_usage: DataUsage,
        data: &[T],
        data_align: u64,
    ) -> Result<Self> {
        /* Staging Buffer */

        let staging_buffer = {
            let create_info = vk::BufferCreateInfo::default()
                .size(data_size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            unsafe { logical_device.create_buffer(&create_info, None) }?
        };

        let staging_buffer_mem_requirements =
            unsafe { logical_device.get_buffer_memory_requirements(staging_buffer) };

        let staging_buffer_memory = {
            let mut memory_type_index: u32 = 0;
            for mt in device_mem_properties.memory_types {
                if (staging_buffer_mem_requirements.memory_type_bits & (1 << memory_type_index)
                    != 0)
                    && mt.property_flags.contains(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )
                {
                    break;
                }

                memory_type_index += 1;
            }

            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(staging_buffer_mem_requirements.size)
                .memory_type_index(memory_type_index);

            unsafe { logical_device.allocate_memory(&allocate_info, None) }?
        };

        unsafe { logical_device.bind_buffer_memory(staging_buffer, staging_buffer_memory, 0) }?;

        let data_ptr = unsafe {
            logical_device.map_memory(
                staging_buffer_memory,
                0,
                staging_buffer_mem_requirements.size,
                vk::MemoryMapFlags::empty(),
            )
        }?;

        let mut staging_align =
            unsafe { util::Align::new(data_ptr, data_align, staging_buffer_mem_requirements.size) };

        staging_align.copy_from_slice(&data);

        unsafe { logical_device.unmap_memory(staging_buffer_memory) };

        /* Storage Buffer */

        let usage_flag = match data_usage {
            DataUsage::VERTEX => vk::BufferUsageFlags::VERTEX_BUFFER,
            DataUsage::INDEX => vk::BufferUsageFlags::INDEX_BUFFER,
        };

        let buffer = {
            let create_info = vk::BufferCreateInfo::default()
                .size(data_size)
                .usage(vk::BufferUsageFlags::TRANSFER_DST | usage_flag)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            unsafe { logical_device.create_buffer(&create_info, None) }?
        };

        let buffer_mem_requirements =
            unsafe { logical_device.get_buffer_memory_requirements(buffer) };

        let buffer_memory = {
            let mut memory_type_index: u32 = 0;
            for mt in device_mem_properties.memory_types {
                if (buffer_mem_requirements.memory_type_bits & (1 << memory_type_index) != 0)
                    && mt
                        .property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    break;
                }

                memory_type_index += 1;
            }

            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(buffer_mem_requirements.size)
                .memory_type_index(memory_type_index);

            unsafe { logical_device.allocate_memory(&allocate_info, None) }?
        };

        unsafe { logical_device.bind_buffer_memory(buffer, buffer_memory, 0) }?;

        self::CommandBuffer::buffer_copy(
            logical_device,
            queue,
            queue_family_index,
            &[data_size],
            &[&staging_buffer],
            &[&buffer],
        )?;

        /* Cleanup */
        unsafe {
            logical_device.destroy_buffer(staging_buffer, None);
            logical_device.free_memory(staging_buffer_memory, None);
        }

        Ok(Self {
            buffer,
            buffer_memory,
        })
    }

    /// Load new data into an existing [`StorageBuffer`]
    ///
    /// Similar to creation, but without storage buffer creation
    #[allow(dead_code)]
    pub fn load<T: Copy>(
        &self,
        logical_device: &ash::Device,
        device_mem_properties: &vk::PhysicalDeviceMemoryProperties,
        queue: &vk::Queue,
        queue_family_index: &u32,
        data_size: u64,
        data: &[T],
        data_align: u64,
    ) -> Result<()> {
        /* Staging Buffer */

        let staging_buffer = {
            let create_info = vk::BufferCreateInfo::default()
                .size(data_size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            unsafe { logical_device.create_buffer(&create_info, None) }?
        };

        let staging_buffer_mem_requirements =
            unsafe { logical_device.get_buffer_memory_requirements(staging_buffer) };

        let staging_buffer_memory = {
            let mut memory_type_index: u32 = 0;
            for mt in device_mem_properties.memory_types {
                if (staging_buffer_mem_requirements.memory_type_bits & (1 << memory_type_index)
                    != 0)
                    && mt.property_flags.contains(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )
                {
                    break;
                }

                memory_type_index += 1;
            }

            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(staging_buffer_mem_requirements.size)
                .memory_type_index(memory_type_index);

            unsafe { logical_device.allocate_memory(&allocate_info, None) }?
        };

        unsafe { logical_device.bind_buffer_memory(staging_buffer, staging_buffer_memory, 0) }?;

        let data_ptr = unsafe {
            logical_device.map_memory(
                staging_buffer_memory,
                0,
                staging_buffer_mem_requirements.size,
                vk::MemoryMapFlags::empty(),
            )
        }?;

        let mut staging_align =
            unsafe { util::Align::new(data_ptr, data_align, staging_buffer_mem_requirements.size) };

        staging_align.copy_from_slice(&data);

        unsafe { logical_device.unmap_memory(staging_buffer_memory) };

        self::CommandBuffer::buffer_copy(
            logical_device,
            queue,
            queue_family_index,
            &[data_size],
            &[&staging_buffer],
            &[&self.buffer],
        )?;

        /* Cleanup */
        unsafe {
            logical_device.destroy_buffer(staging_buffer, None);
            logical_device.free_memory(staging_buffer_memory, None);
        }

        Ok(())
    }
}

//==================================================
//=== Uniform Buffer
//==================================================

pub struct UniformBuffer {
    pub buffers: Vec<vk::Buffer>,
    pub buffers_memory: Vec<vk::DeviceMemory>,
    pub buffers_mem_req: Vec<vk::MemoryRequirements>,
    pub buffers_mapped: Vec<*mut std::ffi::c_void>,
}

impl UniformBuffer {
    pub fn new(
        logical_device: &ash::Device,
        device_mem_properties: &vk::PhysicalDeviceMemoryProperties,
        buffer_count: usize,
        buffer_size: u64,
    ) -> Result<Self> {
        let mut buffers: Vec<vk::Buffer> = Vec::with_capacity(buffer_count);
        let mut buffers_memory: Vec<vk::DeviceMemory> = Vec::with_capacity(buffer_count);
        let mut buffers_mem_req: Vec<vk::MemoryRequirements> = Vec::with_capacity(buffer_count);
        let mut buffers_mapped: Vec<*mut std::ffi::c_void> = Vec::with_capacity(buffer_count);

        for _ in 0..buffer_count {
            let uniform_buffer = {
                let create_info = vk::BufferCreateInfo::default()
                    .size(buffer_size)
                    .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE);

                unsafe { logical_device.create_buffer(&create_info, None) }?
            };

            let uniform_mem_requirements =
                unsafe { logical_device.get_buffer_memory_requirements(uniform_buffer) };

            let uniform_buffer_memory = {
                let mut memory_type_index: u32 = 0;
                for mt in device_mem_properties.memory_types {
                    if (uniform_mem_requirements.memory_type_bits & (1 << memory_type_index) != 0)
                        && mt.property_flags.contains(
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        )
                    {
                        break;
                    }

                    memory_type_index += 1;
                }

                let allocate_info = vk::MemoryAllocateInfo::default()
                    .allocation_size(uniform_mem_requirements.size)
                    .memory_type_index(memory_type_index);

                unsafe { logical_device.allocate_memory(&allocate_info, None) }?
            };

            unsafe { logical_device.bind_buffer_memory(uniform_buffer, uniform_buffer_memory, 0) }?;

            let uniform_mapped = unsafe {
                logical_device.map_memory(
                    uniform_buffer_memory,
                    0,
                    uniform_mem_requirements.size,
                    vk::MemoryMapFlags::empty(),
                )
            }?;

            buffers.push(uniform_buffer);
            buffers_memory.push(uniform_buffer_memory);
            buffers_mem_req.push(uniform_mem_requirements);
            buffers_mapped.push(uniform_mapped);
        }

        Ok(Self {
            buffers,
            buffers_memory,
            buffers_mem_req,
            buffers_mapped,
        })
    }
}
