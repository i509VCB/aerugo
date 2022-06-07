//! Code which should be upstreamed to ash.

use std::{ffi::CStr, mem};

use ash::{prelude::*, vk};

#[derive(Clone)]
pub struct DrmFormatModifierEXT {
    handle: vk::Device,
    fp: vk::ExtImageDrmFormatModifierFn,
}

impl DrmFormatModifierEXT {
    pub fn new(instance: &ash::Instance, device: &ash::Device) -> Self {
        let handle = device.handle();
        let fp = vk::ExtImageDrmFormatModifierFn::load(|name| unsafe {
            mem::transmute(instance.get_device_proc_addr(handle, name.as_ptr()))
        });
        Self { handle, fp }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkGetImageDrmFormatModifierPropertiesEXT.html>
    pub unsafe fn get_image_drm_format_modifier_properties(
        &self,
        image: &vk::Image,
        properties: &mut vk::ImageDrmFormatModifierPropertiesEXT,
    ) -> VkResult<()> {
        (self.fp.get_image_drm_format_modifier_properties_ext)(self.handle, *image, properties).result()
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn get_drm_format_properties_list(
        instance: &ash::Instance,
        pdevice: vk::PhysicalDevice,
        format: vk::Format,
        format_properties: vk::FormatProperties,
    ) -> Vec<vk::DrmFormatModifierPropertiesEXT> {
        let mut list = vk::DrmFormatModifierPropertiesListEXT::default();

        // Need to get number of entries in list and then allocate the vector as needed.
        {
            let mut format_properties_2 = vk::FormatProperties2::builder()
                .format_properties(format_properties)
                .push_next(&mut list);

            instance.get_physical_device_format_properties2(pdevice, format, &mut format_properties_2);
        }

        let mut data = Vec::with_capacity(list.drm_format_modifier_count as usize);

        // Read the number of elements into the vector.
        list.p_drm_format_modifier_properties = data.as_mut_ptr();

        {
            let mut format_properties_2 = vk::FormatProperties2::builder()
                .format_properties(format_properties)
                .push_next(&mut list);

            instance.get_physical_device_format_properties2(pdevice, format, &mut format_properties_2);
        }

        data.set_len(list.drm_format_modifier_count as usize);
        data
    }

    pub const fn name() -> &'static CStr {
        vk::ExtImageDrmFormatModifierFn::name()
    }

    pub fn fp(&self) -> &vk::ExtImageDrmFormatModifierFn {
        &self.fp
    }

    pub fn device(&self) -> vk::Device {
        self.handle
    }
}
