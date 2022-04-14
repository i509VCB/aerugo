use ash::vk;
use smithay::reexports::wayland_server::protocol::wl_shm;

use crate::{
    format::{formats, fourcc_to_vk, fourcc_to_wl},
    vulkan::{
        error::VkError,
        renderer::{Error, ShmFormatInfo, VulkanRenderer},
    },
};

// TODO(i5): There might be an optimization we can make with iGPUs since an iGPU (and some dGPUs) will
// share memory, meaning device memory is host visible (VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT). In
// theory it would be possible to then instead us `vkMapMemory` and entirely bypass the staging
// buffer. Not 100% sure if this would affect the required flags in the future. For the sake of
// maximum compatibility, a staging buffer will always work.

/// Features a format must support in order to be used as a texture format.
///
/// A format which supports these features may be used to import memory or SHM buffers.
pub(crate) const TEXTURE_FEATURES: vk::FormatFeatureFlags = {
    // TODO: Replace the conversion to as_raw when `impl const` is stabilized and ash uses it for bitwise
    // operations on flags.
    let bits =
        // A texture must support being the source and destination of image transfer commands.
        //
        // This is required for texture import and export.
        //
        // TODO: Distinguish between texture formats that may be imported and exported, as these may be
        // theoretically different. This could mean Argb8888 is allowed for import but not export?
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

impl VulkanRenderer {
    /// Tests which wl_shm formats are supported for the renderer.
    ///
    /// This function will go through the list of known formats and test if the texture image usage and format
    /// features are supported.
    ///
    /// # Errors
    ///
    /// This function will return [`Err`] if the mandatory wl_shm formats are not supported or some other
    /// error occurs.
    pub(super) fn init_shm_formats(&mut self) -> Result<(), Error> {
        let instance = self.device.instance.raw();
        let phy = self.device.phy;

        for format in formats() {
            if let Some((vk_format, _)) = fourcc_to_vk(format) {
                let shm = match fourcc_to_wl(format) {
                    Some(format) => format,
                    None => continue,
                };

                let mut image_format_properties2 = vk::ImageFormatProperties2::builder();
                let format_info = vk::PhysicalDeviceImageFormatInfo2::builder()
                    .format(vk_format)
                    .tiling(vk::ImageTiling::OPTIMAL)
                    .ty(vk::ImageType::TYPE_2D)
                    .usage(TEXTURE_USAGE);

                let image_format_properties = match unsafe {
                    instance.get_physical_device_image_format_properties2(
                        phy,
                        &format_info,
                        &mut image_format_properties2,
                    )
                } {
                    Ok(_) => image_format_properties2.image_format_properties,
                    Err(vk::Result::ERROR_FORMAT_NOT_SUPPORTED) => continue,
                    Err(result) => return Err(VkError::from(result).into()),
                };

                // Check if the format supports the texture usage feature flags.
                let mut format_properties2 = vk::FormatProperties2::builder();
                unsafe { instance.get_physical_device_format_properties2(phy, vk_format, &mut format_properties2) };

                if format_properties2
                    .format_properties
                    .optimal_tiling_features
                    .contains(TEXTURE_FEATURES)
                {
                    self.formats.shm_format_info.push(ShmFormatInfo {
                        shm,
                        vk: vk_format,
                        max_extent: vk::Extent2D {
                            width: image_format_properties.max_extent.width,
                            height: image_format_properties.max_extent.height,
                        },
                    });

                    self.formats.shm_formats.push(shm);
                }
            }
        }

        // Ensure the required wl_shm formats are available
        if !self
            .formats
            .shm_formats
            .iter()
            .any(|format| format == &wl_shm::Format::Argb8888 || format == &wl_shm::Format::Xrgb8888)
        {
            return Err(Error::MissingMandatoryFormats);
        }

        Ok(())
    }
}
