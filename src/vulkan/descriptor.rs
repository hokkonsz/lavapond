// extern
use anyhow::{Context, Result};
use ash::vk;

//==================================================
//=== Descriptor
//==================================================

pub struct Descriptor {
    pub set_layout: vk::DescriptorSetLayout,
    pub pool: vk::DescriptorPool,
    pub sets: Vec<vk::DescriptorSet>,
}

impl Descriptor {
    /// Creates a new [`Descriptor`]
    pub fn new(logical_device: &ash::Device, max_frames_inflight: usize) -> Result<Self> {
        let set_layout = {
            let layout_binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX);

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(std::slice::from_ref(&layout_binding));

            unsafe { logical_device.create_descriptor_set_layout(&create_info, None) }?
        };

        let pool = {
            let pool_size =
                vk::DescriptorPoolSize::builder().descriptor_count(max_frames_inflight as u32);

            let create_info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(std::slice::from_ref(&pool_size))
                .max_sets(max_frames_inflight as u32);

            unsafe { logical_device.create_descriptor_pool(&create_info, None) }?
        };

        let sets = {
            let set_layouts = vec![set_layout; max_frames_inflight];

            let allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&set_layouts);

            unsafe { logical_device.allocate_descriptor_sets(&allocate_info) }?
        };

        Ok(Self {
            set_layout,
            pool,
            sets,
        })
    }

    /// Updates the current descriptor sets with buffer data
    pub fn update_descriptor_sets(
        &self,
        logical_device: &ash::Device,
        max_frames_inflight: usize,
        buffers: &Vec<vk::Buffer>,
        data_size: u64,
    ) -> Result<()> {
        for i in 0..max_frames_inflight {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(
                    *buffers
                        .get(i)
                        .context("Descriptor Bufer: 'buffer' index out of bounds")?,
                )
                .offset(0)
                .range(data_size);

            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(
                    *self
                        .sets
                        .get(i)
                        .context("Write Descriptor Set: 'dst_set' index out of bounds")?,
                )
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info));

            unsafe {
                logical_device.update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[])
            };
        }

        Ok(())
    }
}
