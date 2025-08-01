use anyhow::Result;
use ash::{ext, khr, vk};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{borrow::Cow, ffi::CStr};
use winit::window;

//==================================================
//=== Debug Messenger
//==================================================

pub struct DebugExtension {
    pub loader: ext::debug_utils::Instance,
    pub messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugExtension {
    /// Creates a new [`DebugExtension`]
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Result<Self> {
        let loader = ext::debug_utils::Instance::new(entry, instance);

        let messenger = {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            unsafe { loader.create_debug_utils_messenger(&debug_info, None) }?
        };

        Ok(Self { loader, messenger })
    }
}

/// Callback function for debug messenger
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = unsafe { *p_callback_data };
    let message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        unsafe { CStr::from_ptr(callback_data.p_message).to_string_lossy() }
    };

    println!(
        "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
    );

    vk::FALSE
}

//==================================================
//=== Surface
//==================================================

pub struct SurfaceExtension {
    pub loader: khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
}

impl SurfaceExtension {
    /// Creates a new [`SurfaceExtension`]
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &window::Window,
    ) -> Result<Self> {
        let loader = khr::surface::Instance::new(entry, instance);

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
        }?;

        Ok(Self { loader, surface })
    }
}

//==================================================
//=== Swapchain
//==================================================

pub struct SwapchainExtension {
    pub loader: khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
}

impl SwapchainExtension {
    /// Creates a new [`SwapchainExtension`]
    pub fn new(
        instance: &ash::Instance,
        logical_device: &ash::Device,
        physical_device: &vk::PhysicalDevice,
        surface_ext: &SurfaceExtension,
        window: &winit::window::Window,
    ) -> Result<Self> {
        let loader = khr::swapchain::Device::new(instance, logical_device);

        let swapchain = {
            let (min_image_count, pre_transform) = {
                let caps = unsafe {
                    surface_ext.loader.get_physical_device_surface_capabilities(
                        *physical_device,
                        surface_ext.surface,
                    )
                }?;
                let mut count = caps.min_image_count + 1;

                if caps.max_image_count > 0 && count > caps.max_image_count {
                    count = caps.max_image_count;
                }

                (count, caps.current_transform)
            };

            let image_extent = vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            };

            // TODO! -> This is too strict/error prone right now, better to supplement with queried data
            // TODO! -> Check for defaults
            let create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface_ext.surface)
                .min_image_count(min_image_count)
                .image_format(vk::Format::B8G8R8A8_SRGB)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(image_extent)
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

            // TODO! -> STATUS_STACK_BUFFER_OVERRUN Error
            unsafe { loader.create_swapchain(&create_info, None) }?
        };

        Ok(Self { loader, swapchain })
    }
}
