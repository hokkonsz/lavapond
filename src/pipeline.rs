use anyhow::{Context, Ok, Result};
use ash::vk;
use std::ffi::CStr;
use utils::color::Color;

//==================================================
//=== Graphics Pipeline
//==================================================

pub struct GraphicsPipeline {
    pub layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub pipeline: vk::Pipeline,
}

impl GraphicsPipeline {
    /// Creates a new [`GraphicsPipeline`]
    pub fn new(
        logical_device: &ash::Device,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        viewport: &vk::Viewport,
        scissor: &vk::Rect2D,
        vertex_stride: u32,
        push_constant_ranges: &vk::PushConstantRange,
    ) -> Result<Self> {
        /* Pipeline Stages */

        let shader_mod_vert = {
            let code = std::fs::read("res/shaders/spirv/shader.vert.spv")?;

            let create_info = vk::ShaderModuleCreateInfo::default()
                .code(bytemuck::try_cast_slice(code.as_slice())?);

            unsafe { logical_device.create_shader_module(&create_info, None) }?
        };

        let vert_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(shader_mod_vert)
            .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") });

        let shader_mod_frag = {
            let code = std::fs::read("res/shaders/spirv/shader.frag.spv")?;

            let create_info = vk::ShaderModuleCreateInfo::default()
                .code(bytemuck::try_cast_slice(code.as_slice())?);

            unsafe { logical_device.create_shader_module(&create_info, None) }?
        };

        let frag_shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(shader_mod_frag)
            .name(CStr::from_bytes_with_nul(b"main\0")?);

        let shader_stages = [vert_shader_stage, frag_shader_stage];

        /* Pipeline States */

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let vertex_binding_descriptions = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(vertex_stride)
            .input_rate(vk::VertexInputRate::VERTEX);

        let vertex_attribute_descriptions = [
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset((std::mem::size_of::<Color>()) as u32),
        ];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&vertex_binding_descriptions))
            .vertex_attribute_descriptions(&vertex_attribute_descriptions);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0);

        let color_blend_attachment_state = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(std::slice::from_ref(&color_blend_attachment_state));

        /* Render- & Subpasses */

        let color_attachment = vk::AttachmentDescription::default()
            .format(vk::Format::B8G8R8A8_SRGB)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0) // <- Index of attachment descriptor
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref));

        let subpass_dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let render_pass = {
            let create_info = vk::RenderPassCreateInfo::default()
                .attachments(std::slice::from_ref(&color_attachment))
                .subpasses(std::slice::from_ref(&subpass))
                .dependencies(std::slice::from_ref(&subpass_dependency));

            unsafe { logical_device.create_render_pass(&create_info, None)? }
        };

        /* Pipeline Finalization */

        let layout = {
            let create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(std::slice::from_ref(&descriptor_set_layout))
                .push_constant_ranges(std::slice::from_ref(&push_constant_ranges));

            unsafe { logical_device.create_pipeline_layout(&create_info, None) }?
        };

        let pipeline = {
            let create_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stages)
                .input_assembly_state(&input_assembly_state)
                .vertex_input_state(&vertex_input_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                //.depth_stencil_state(depth_stencil_state)
                .color_blend_state(&color_blend_state)
                .dynamic_state(&dynamic_state)
                .layout(layout)
                .render_pass(render_pass)
                .subpass(0);

            unsafe {
                logical_device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&create_info),
                    None,
                )
            }
        }
        // TODO! Better/Nicer way?
        .into_iter()
        .next()
        .context("Could not create the graphics pipeline")?
        .into_iter()
        .next()
        .context("Could not find the graphics pipeline")?;

        /* Pipeline Cleanup */

        unsafe {
            logical_device.destroy_shader_module(shader_mod_frag, None);
            logical_device.destroy_shader_module(shader_mod_vert, None);
        };

        Ok(Self {
            layout,
            render_pass,
            pipeline,
        })
    }
}
