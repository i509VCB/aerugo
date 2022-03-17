use ash::vk;
use smithay::{backend::allocator, reexports::wayland_server::protocol::wl_shm};

use crate::{
    format::{formats, fourcc_to_vk, fourcc_to_wl},
    vulkan::{
        error::VkError,
        renderer::{Error, ShmFormatInfo, VulkanRenderer},
    },
};

/// Features a format must support in order to be used as a texture format.
///
/// A format which supports these features may be used to import memory or SHM buffers.
pub(crate) const TEXTURE_FEATURES: vk::FormatFeatureFlags = {
    // TODO: Replace the conversion to as_raw when `impl const` is stabilized and ash uses it for bitwise
    // operations on flags.
    let bits =
        // Transfer features must be supported since we currently use a staging buffer to upload texture data.
        //
        // TODO(i5): There might be an optimization we can make with iGPUs since an iGPU (and some dGPUs) will
        // share memory, meaning device memory is host visible (VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT). In
        // theory it would be possible to then instead us `vkMapMemory` and entirely bypass the staging
        // buffer. Not 100% sure if this would affect the required flags in the future. For the sake of
        // maximum compatibility, a staging buffer will always work.
        vk::FormatFeatureFlags::TRANSFER_SRC.as_raw()
        | vk::FormatFeatureFlags::TRANSFER_DST.as_raw()
        | vk::FormatFeatureFlags::SAMPLED_IMAGE.as_raw();
    vk::FormatFeatureFlags::from_raw(bits)
};

pub(crate) const TEXTURE_USAGE: vk::ImageUsageFlags = {
    // TODO: Replace the conversion to as_raw when `impl const` is stabilized and ash uses it for bitwise
    // operations on flags.
    let bits = vk::ImageUsageFlags::SAMPLED.as_raw() | vk::ImageUsageFlags::TRANSFER_DST.as_raw();
    vk::ImageUsageFlags::from_raw(bits)
};

/// Features a format must support in order to be used for rendering.
pub(crate) const RENDER_FEATURES: vk::FormatFeatureFlags = {
    // TODO: Replace the conversion to as_raw when `impl const` is stabilized and ash uses it for bitwise
    // operations on flags.
    let bits =
        vk::FormatFeatureFlags::COLOR_ATTACHMENT.as_raw() | vk::FormatFeatureFlags::COLOR_ATTACHMENT_BLEND.as_raw();
    vk::FormatFeatureFlags::from_raw(bits)
};

pub(crate) const RENDER_USAGE: vk::ImageUsageFlags = vk::ImageUsageFlags::COLOR_ATTACHMENT;

/// Features a format must support in order to be used as a dmabuf texture format.
///
/// A format which supports these features may be used to import dmabufs of the same format.
pub(crate) const DMA_TEXTURE_FEATURES: vk::FormatFeatureFlags = vk::FormatFeatureFlags::SAMPLED_IMAGE;

pub(crate) const DMA_TEXTURE_USAGE: vk::ImageUsageFlags = vk::ImageUsageFlags::SAMPLED;

/// # Safety
///
/// The physical device must support the `VK_EXT_image_drm_format_modifier` extension.
pub(crate) unsafe fn get_format_modifiers(
    instance: &ash::Instance,
    phy: vk::PhysicalDevice,
    format: vk::Format,
) -> Vec<vk::DrmFormatModifierPropertiesEXT> {
    // First we need to query how many entries are available.
    let mut formats_ext = vk::DrmFormatModifierPropertiesListEXT::builder();
    let mut properties2 = vk::FormatProperties2::builder().push_next(&mut formats_ext);

    // SAFETY: VK_EXT_image_drm_format_modifier is enabled
    // Null pointer for pDrmFormatModifierProperties is safe when obtaining count.
    unsafe {
        instance.get_physical_device_format_properties2(phy, format, &mut properties2);
    }

    // Immediately end the mutable borrow on `formats_ext` in order to ensure the formats_ext is accessible.
    drop(properties2);

    let modifier_count = formats_ext.drm_format_modifier_count as usize;
    let mut modifiers = Vec::with_capacity(modifier_count);
    formats_ext = formats_ext.drm_format_modifier_properties(&mut modifiers[..]);
    // FIXME: Ash sets `drm_format_modifier_count`, but len() returns zero, we have the length so we need to
    // set it again.
    formats_ext.drm_format_modifier_count = modifier_count as _;

    // Now we can create a new struct since the fields are filled in on the extension.
    let mut properties2 = vk::FormatProperties2::builder().push_next(&mut formats_ext);

    // Initialize the value
    unsafe {
        instance.get_physical_device_format_properties2(phy, format, &mut properties2);

        // SAFETY: Elements from 0..len() were just initialized.
        modifiers.set_len(modifier_count);
    }

    modifiers
}

pub(crate) unsafe fn get_dma_image_format_properties(
    instance: &ash::Instance,
    phy: vk::PhysicalDevice,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
) -> Result<Option<(vk::ExternalMemoryProperties, vk::ImageFormatProperties)>, VkError> {
    let external_memory_properties = vk::ExternalMemoryProperties::builder()
        // Must be able to import a dmabuf matching said format.
        .external_memory_features(vk::ExternalMemoryFeatureFlags::IMPORTABLE)
        // Format must be usable in a dmabuf image.
        .compatible_handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        // .export_from_imported_handle_types(export_from_imported_handle_types) // TODO
        .build();

    let mut external_image_format_properties =
        vk::ExternalImageFormatProperties::builder().external_memory_properties(external_memory_properties);
    let mut image_format_properties_builder =
        vk::ImageFormatProperties2::builder().push_next(&mut external_image_format_properties);

    let image_format_info = vk::PhysicalDeviceImageFormatInfo2::builder()
        .format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .ty(vk::ImageType::TYPE_2D)
        .usage(usage);

    if let Err(result) = unsafe {
        instance.get_physical_device_image_format_properties2(
            phy,
            &image_format_info,
            &mut image_format_properties_builder,
        )
    } {
        if result == vk::Result::ERROR_FORMAT_NOT_SUPPORTED {
            // Unsupported format
            Ok(None)
        } else {
            Err(result.into())
        }
    } else {
        let image_format_properties = image_format_properties_builder.image_format_properties;

        Ok(Some((
            external_image_format_properties.external_memory_properties,
            image_format_properties,
        )))
    }
}

pub(crate) unsafe fn get_image_format_properties(
    instance: &ash::Instance,
    phy: vk::PhysicalDevice,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    drm_format_info: Option<&mut vk::PhysicalDeviceImageDrmFormatModifierInfoEXT>,
) -> Result<Option<vk::ImageFormatProperties>, VkError> {
    let tiling = if drm_format_info.is_some() {
        // VUID-VkPhysicalDeviceImageFormatInfo2-tiling-02249
        vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT
    } else {
        vk::ImageTiling::OPTIMAL
    };

    let mut format_properties = vk::ImageFormatProperties2::builder();
    let mut format_info = vk::PhysicalDeviceImageFormatInfo2::builder()
        .format(format)
        .tiling(tiling)
        .ty(vk::ImageType::TYPE_2D)
        .usage(usage);

    if let Some(drm_format_info) = drm_format_info {
        format_info = format_info.push_next(drm_format_info);
    }

    if let Err(result) =
        unsafe { instance.get_physical_device_image_format_properties2(phy, &format_info, &mut format_properties) }
    {
        if result != vk::Result::ERROR_FORMAT_NOT_SUPPORTED {
            Err(result.into())
        } else {
            // Unsupported format
            Ok(None)
        }
    } else {
        Ok(Some(format_properties.image_format_properties))
    }
}

impl VulkanRenderer {
    pub(crate) fn load_formats(&mut self) -> Result<(), Error> {
        let instance = self.device.instance.raw();
        let phy = self.device.phy;

        for format in formats() {
            if let Some((vk_format, _)) = fourcc_to_vk(format) {
                // SAFETY: VK_EXT_image_drm_format_modifier is available.
                let modifiers = unsafe { get_format_modifiers(instance, phy, vk_format) };

                // Check if the modifiers support specific types of usages.
                for modifier in modifiers {
                    // Rendering
                    if modifier.drm_format_modifier_tiling_features.contains(RENDER_FEATURES) {
                        // External memory must also support the tiling features that rendering does.
                        if let Some(external_memory_features) =
                            unsafe { get_dma_image_format_properties(instance, phy, vk_format, RENDER_USAGE) }?
                        {
                            if external_memory_features
                                .0
                                .external_memory_features
                                .contains(vk::ExternalMemoryFeatureFlags::IMPORTABLE)
                            {
                                self.dma_render_formats.push(allocator::Format {
                                    code: format,
                                    modifier: allocator::Modifier::from(modifier.drm_format_modifier),
                                });
                            }
                        }
                    }

                    // Dmabuf
                    if modifier
                        .drm_format_modifier_tiling_features
                        .contains(DMA_TEXTURE_FEATURES)
                    {
                        if let Some(external_memory_features) =
                            unsafe { get_dma_image_format_properties(instance, phy, vk_format, DMA_TEXTURE_USAGE) }?
                        {
                            if external_memory_features
                                .0
                                .external_memory_features
                                .contains(vk::ExternalMemoryFeatureFlags::IMPORTABLE)
                            {
                                self.dma_texture_formats.push(allocator::Format {
                                    code: format,
                                    modifier: allocator::Modifier::from(modifier.drm_format_modifier),
                                });
                            }
                        }
                    }
                }

                // Memory
                if let Some(image_format_properties) =
                    unsafe { get_image_format_properties(instance, phy, vk_format, TEXTURE_USAGE, None) }?
                {
                    let mut format_properties = vk::FormatProperties2::default();
                    unsafe { instance.get_physical_device_format_properties2(phy, vk_format, &mut format_properties) };

                    if format_properties
                        .format_properties
                        .optimal_tiling_features
                        .contains(TEXTURE_FEATURES)
                    {
                        if let Some(shm) = fourcc_to_wl(format) {
                            self.shm_format_info.push(ShmFormatInfo {
                                shm,
                                vk: vk_format,
                                max_extent: vk::Extent2D {
                                    width: image_format_properties.max_extent.width,
                                    height: image_format_properties.max_extent.height,
                                },
                            });

                            self.shm_formats.push(shm);
                        }
                    }
                }
            }
        }

        // Ensure the shm renderer has the mandatory formats.
        if !self
            .shm_formats
            .iter()
            .any(|format| format == &wl_shm::Format::Argb8888 || format == &wl_shm::Format::Xrgb8888)
        {
            return Err(Error::MissingRequiredFormats);
        }

        Ok(())
    }
}
