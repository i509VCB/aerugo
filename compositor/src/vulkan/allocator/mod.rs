mod upstream;

use std::{fmt, sync::Arc, convert::identity};

use ash::{extensions::khr::ExternalMemoryFd, vk};
use bitflags::bitflags;
use smithay::{
    backend::allocator::{
        dmabuf::{AsDmabuf, Dmabuf},
        Allocator, Buffer, Format, Fourcc, Modifier,
    },
    utils::{Buffer as BufferCoord, Size},
};

use self::upstream::DrmFormatModifierEXT;

use super::{
    device::{Device, DeviceHandle},
    error::VkError,
};

bitflags! {
    /// Flags to indicate the intended usage for the buffer.
    ///
    /// Use [`VulkanAllocator::is_format_supported`] to check if the combination of format and usage flags
    /// are supported.
    pub struct ImageUsageFlags: vk::Flags {
        /// The image may be the source of a transfer command.
        const TRANSFER_SRC = vk::ImageUsageFlags::TRANSFER_SRC.as_raw();

        /// The image may be the destination of a transfer command.
        const TRANSFER_DST = vk::ImageUsageFlags::TRANSFER_DST.as_raw();

        /// Image may be sampled in a shader.
        const SAMPLED = vk::ImageUsageFlags::SAMPLED.as_raw();

        /// The image may be used in a color attachment.
        const COLOR_ATTACHMENT = vk::ImageUsageFlags::COLOR_ATTACHMENT.as_raw();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VulkanAllocatorError {
    /// The device was not created with the required device extensions.
    #[error("required extensions are not enabled")]
    MissingRequiredExtensions,

    #[error("the requested format is not supported")]
    UnsupportedFormat,

    #[error("the buffer was created with an invalid size")]
    InvalidSize,

    #[error("no modifiers specified")]
    NoModifiers,

    /// A Vulkan API error.
    #[error(transparent)]
    Vk(#[from] VkError),
}

pub struct VulkanAllocator {
    /// All supported formats.
    ///
    /// Note this does not guarantee a specific image usage is valid with said format. Further checks are
    /// needed to ensure an image usage is valid with said format.
    formats: Vec<Format>,

    // TODO: Upstream to ash
    drm_format_modifier: DrmFormatModifierEXT,

    /// If [`Some`], then the device supports [`Dmabuf`] import or export.
    external_memory_fd: Option<ExternalMemoryFd>,

    device_handle: Arc<DeviceHandle>,
}

impl fmt::Debug for VulkanAllocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VulkanAllocator")
            .field("device_handle", &self.device_handle)
            .finish_non_exhaustive()
    }
}

impl VulkanAllocator {
    /// Extensions a device must enable to use a [`VulkanAllocator`].
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub const fn required_device_extensions() -> &'static [&'static str] {
        &[
            "VK_EXT_image_drm_format_modifier",
            "VK_KHR_image_format_list", // Or Vulkan 1.2
        ]
    }

    /// Extensions a device must enable to use all features provided by a [`VulkanAllocator`].
    ///
    /// This is a superset of the [`required device extensions`](VulkanAllocator::required_device_extensions).
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub const fn optimal_device_extensions() -> &'static [&'static str] {
        &[
            // Required extensions
            "VK_EXT_image_drm_format_modifier",
            "VK_KHR_image_format_list", // Or Vulkan 1.2
            // Optimal extensions
            "VK_KHR_external_memory_fd",
            "VK_EXT_external_memory_dma_buf",
        ]
    }

    pub fn new(device: &Device) -> Result<VulkanAllocator, VulkanAllocatorError> {
        if !Self::required_device_extensions()
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            return Err(VulkanAllocatorError::MissingRequiredExtensions);
        }

        // Test if the renderer supports Dmabuf external memory.
        let external_memory_fd = if Self::optimal_device_extensions()
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            Some(ExternalMemoryFd::new(device.instance().raw(), device.raw()))
        } else {
            None
        };

        let drm_format_modifier = DrmFormatModifierEXT::new(device.instance().raw(), device.raw());
        let mut allocator = VulkanAllocator {
            formats: Vec::new(),
            drm_format_modifier,
            external_memory_fd,
            device_handle: device.handle(),
        };

        // Get a list of supported image formats.
        allocator.load_formats();

        Ok(allocator)
    }

    pub fn create_buffer_with_usage(
        &self,
        width: u32,
        height: u32,
        fourcc: Fourcc,
        modifiers: &[Modifier],
        usage: ImageUsageFlags,
    ) -> Result<VulkanImage, VulkanAllocatorError> {
        let format = match crate::format::fourcc_to_vk(fourcc) {
            Some((format, _)) => format,
            None => return Err(VulkanAllocatorError::UnsupportedFormat),
        };

        // VUID-VkImageCreateInfo-extent-00944, VUID-VkImageCreateInfo-extent-00945
        if width == 0 || height == 0 {
            return Err(VulkanAllocatorError::InvalidSize);
        }

        // Some usage flags require specific extensions or device features. We do not allow these right now.
        let usage = vk::ImageUsageFlags::from_raw(usage.bits());

        let modifiers = modifiers
            .iter()
            .copied()
            .filter(|modifier| {
                let info = unsafe {
                    self.get_format_info(
                        Format {
                            code: fourcc,
                            modifier: *modifier,
                        },
                        usage,
                    )
                }
                .ok()
                .flatten();

                // Filter modifiers where the required image creation limits are not met
                // (VUID-VkImageDrmFormatModifierListCreateInfoEXT-pDrmFormatModifiers-02263).
                info.filter(|properties| {
                    // VUID-VkImageCreateInfo-extent-02252
                    properties.max_extent.width >= width
                        // VUID-VkImageCreateInfo-extent-02253
                        && properties.max_extent.height >= height
                        // VUID-VkImageCreateInfo-extent-02254
                        // VUID-VkImageCreateInfo-extent-00946
                        // VUID-VkImageCreateInfo-imageType-00957
                        && properties.max_extent.depth >= 1
                        // VUID-VkImageCreateInfo-samples-02258
                        && properties.sample_counts.contains(vk::SampleCountFlags::TYPE_1)
                })
                .is_some()
            })
            .map(Into::<u64>::into)
            .collect::<Vec<_>>();

        // VUID-VkImageDrmFormatModifierListCreateInfoEXT-drmFormatModifierCount-arraylength
        if modifiers.is_empty() {
            return Err(VulkanAllocatorError::NoModifiers);
        }

        // Specify one of the modifiers must be used when creating the image.
        let mut image_format_drm_modifier_list_create_info_ext =
            vk::ImageDrmFormatModifierListCreateInfoEXT::builder().drm_format_modifiers(&modifiers[..]);
        let mut external_memory_image_create_info =
            vk::ExternalMemoryImageCreateInfo::builder().handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let mut image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                // VUID-VkImageCreateInfo-extent-00946
                // VUID-VkImageCreateInfo-imageType-00957
                depth: 1,
            })
            // VUID-VkImageCreateInfo-samples-parameter
            .samples(vk::SampleCountFlags::TYPE_1)
            // VUID-VkImageCreateInfo-mipLevels-00947
            .mip_levels(1) //
            // VUID-VkImageCreateInfo-arrayLayers-00948
            .array_layers(1) //
            // VUID-VkImageCreateInfo-pNext-02262
            .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            // VUID-VkImageCreateInfo-usage-requiredbitmask
            .usage(usage)
            // VUID-VkImageCreateInfo-initialLayout-00993
            .initial_layout(vk::ImageLayout::UNDEFINED)
            // VUID-VkImageCreateInfo-tiling-02261
            .push_next(&mut image_format_drm_modifier_list_create_info_ext);

        // If the device supports dmabuf external memory, suggest that the image should be backend by Dmabuf
        // external memory.
        if self.external_memory_fd.is_some() {
            image_create_info = image_create_info.push_next(&mut external_memory_image_create_info);
        }

        let device = self.device_handle.raw();
        let image = unsafe { device.create_image(&image_create_info, None) }.map_err(VkError::from)?;

        // In order to store a complete format, get the modifier info of the image.
        let mut image_modifier_properties = vk::ImageDrmFormatModifierPropertiesEXT::builder();

        if let Err(err) = unsafe {
            self.drm_format_modifier
                .get_image_drm_format_modifier_properties(&image, &mut image_modifier_properties)
        } {
            // Destroy the image to prevent a memory leak
            unsafe { device.destroy_image(image, None) };

            return Err(VkError::from(err).into());
        }

        let modifier = Modifier::from(image_modifier_properties.drm_format_modifier);
        let format = Format { code: fourcc, modifier };

        Ok(VulkanImage(Arc::new(ImageInner {
            image,
            format,
            width,
            height,
            memory: None,
            device_handle: self.device_handle.clone(),
        })))
    }

    // TODO: Should this take the image dimensions? Vulkan states there is a maximum extent for a format.
    pub fn is_format_supported(
        &self,
        format: Format,
        usage: ImageUsageFlags
    ) -> bool {
        unsafe { self.get_format_info(format, vk::ImageUsageFlags::from_raw(usage.bits())) }
            .ok()
            .is_some()
    }

    // TODO: Do we need a create_buffer function that takes a vk::Format. Probably not because Vulkan itself
    //       is colorspace agnostic until you try to use the image for something that is done in a specific
    //       colorspace (such as presentation and sampling). DRM formats and modifiers do not care about the
    //       colorspace, applications and presentation hardware do.

    // TODO: Import (if possible)
}

impl Allocator<VulkanImage> for VulkanAllocator {
    type Error = VulkanAllocatorError;

    fn create_buffer(
        &mut self,
        width: u32,
        height: u32,
        fourcc: Fourcc,
        modifiers: &[Modifier],
    ) -> Result<VulkanImage, Self::Error> {
        // TODO: The default usage flags are probably not correct.
        self.create_buffer_with_usage(width, height, fourcc, modifiers, ImageUsageFlags::SAMPLED)
    }
}

#[derive(Debug, Clone)]
pub struct VulkanImage(Arc<ImageInner>);

impl VulkanImage {
    /// Returns the underlying [`Image`](vk::Image).
    pub fn image(&self) -> &vk::Image {
        &self.0.image
    }
}

impl Buffer for VulkanImage {
    fn width(&self) -> u32 {
        self.0.width
    }

    fn height(&self) -> u32 {
        self.0.height
    }

    fn size(&self) -> Size<i32, BufferCoord> {
        (self.0.width as i32, self.0.height as i32).into()
    }

    fn format(&self) -> Format {
        self.0.format
    }
}

impl AsDmabuf for VulkanImage {
    type Error = ImageConvertError;

    fn export(&self) -> Result<Dmabuf, Self::Error> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImageConvertError {
    /// The device does not support [`Dmabuf`] import or export.
    ///
    /// This may occur if the device was not created with the
    /// [`optimal device extensions`](VulkanAllocator::optimal_device_extensions).
    #[error("the device does not support dmabuf import or export")]
    NotSupported,
}

impl VulkanAllocator {
    fn load_formats(&mut self) {
        let instance = self.device_handle.instance();
        let instance = instance.raw();
        let physical = *self.device_handle.phy();

        for fourcc in crate::format::formats() {
            if let Some((format, _)) = crate::format::fourcc_to_vk(fourcc) {
                // First get a list of DRM format modifiers supported for a format.
                // TODO: Any buffer features?
                let format_properties = vk::FormatProperties::builder().build();

                let modifier_properties = unsafe {
                    DrmFormatModifierEXT::get_drm_format_properties_list(instance, physical, format, format_properties)
                };

                // TODO: Are the `drm_format_modifier_tiling_features` useful by chance?
                for format_modifier_properties in modifier_properties {
                    // We could get all the information about the images that could be created using the
                    // format + modifier combination, but there are too many valid image usage combinations to
                    // precalculate that. Instead this check will be done at buffer creation time or if the
                    // user checks given some specified image usage flags.
                    self.formats.push(Format {
                        code: fourcc,
                        modifier: Modifier::from(format_modifier_properties.drm_format_modifier),
                    });
                }
            }
        }
    }

    /// Returns image format properties of a format.
    ///
    /// # Safety
    ///
    /// Image usage flags must not require any specific extensions. The values in [`ImageUsageFlags`] (not the
    /// ash one) satisfy this requirement.
    unsafe fn get_format_info(
        &self,
        format: Format,
        usage: ash::vk::ImageUsageFlags,
    ) -> Result<Option<vk::ImageFormatProperties>, VulkanAllocatorError> {
        // We need to understand the format.
        if !self.formats.contains(&format) {
            return Ok(None);
        }

        let vk_format = crate::format::fourcc_to_vk(format.code)
            .expect("Fourcc must be convertible to Vulkan if understood")
            .0;

        let physical = *self.device_handle.phy();
        let instance = self.device_handle.instance();
        let instance = instance.raw();

        // If we understand the format, determine whether the usage flags are valid for the code + modifier
        // combination.
        let mut image_drm_format_modifier_info = vk::PhysicalDeviceImageDrmFormatModifierInfoEXT::builder()
            .drm_format_modifier(format.modifier.into())
            // No queue specified since sharing mode is exclusive
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let format_info = vk::PhysicalDeviceImageFormatInfo2::builder()
            .format(vk_format)
            .ty(vk::ImageType::TYPE_2D)
            .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            .usage(usage)
            .flags(vk::ImageCreateFlags::empty())
            // VUID-VkPhysicalDeviceImageFormatInfo2-tiling-02249
            .push_next(&mut image_drm_format_modifier_info);

        let mut image_format_properties = vk::ImageFormatProperties2::builder();

        // Per VUID-vkGetPhysicalDeviceImageFormatProperties-tiling-02248
        // > Use vkGetPhysicalDeviceImageFormatProperties2 instead
        match unsafe {
            instance.get_physical_device_image_format_properties2(physical, &format_info, &mut image_format_properties)
        } {
            Ok(_) => Ok(Some(image_format_properties.image_format_properties)),

            // Unsupported format + usage combination
            Err(vk::Result::ERROR_FORMAT_NOT_SUPPORTED) => Ok(None),

            Err(err) => Err(VkError::from(err).into()),
        }
    }
}

#[derive(Debug)]
struct ImageInner {
    /// The underlying image.
    image: vk::Image,
    format: Format,
    width: u32,
    height: u32,
    /// Device memory associated with the image.
    ///
    /// This field is [`Some`] when the image was imported from a [`Dmabuf`].
    memory: Option<vk::DeviceMemory>,
    /// The device which created or imported this image.
    ///
    /// This field is here to ensure the image cannot outlive the device.
    device_handle: Arc<DeviceHandle>,
}

impl Drop for ImageInner {
    fn drop(&mut self) {
        let device = self.device_handle.raw();

        unsafe { device.destroy_image(self.image, None) };

        if let Some(memory) = self.memory {
            unsafe { device.free_memory(memory, None) };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use slog::Drain;
    use smithay::backend::allocator::{Allocator, Buffer};

    use crate::vulkan::{allocator::ImageUsageFlags, device::Device, instance::Instance, VALIDATION_LAYER_NAME};

    use super::VulkanAllocator;

    /// This test asserts the device extensions specified in [`VulkanAllocator::required_device_extensions`]
    /// are also contained by [`VulkanAllocator::optimal_device_extensions`].
    #[test]
    fn optimal_extensions_superset() {
        assert_eq!(
            VulkanAllocator::required_device_extensions()
                .iter()
                // Remove the extension from the iterator if the optimal extensions also contain it.
                .filter(|extension| !VulkanAllocator::optimal_device_extensions().contains(*extension))
                .collect::<Vec<&&str>>(),
            Vec::<&&str>::new(),
            "Optimal device extensions must contain all required device extensions",
        );
    }

    #[test]
    fn allocate_image_test() {
        let logger = slog::Logger::root(Mutex::new(slog_term::term_full().fuse()).fuse(), slog::o!());

        let instance = unsafe { Instance::builder().layer(VALIDATION_LAYER_NAME).build(logger) }.expect("instance");

        // Try to find a device with optimal settings first.
        let (physical, extensions) = match instance.enumerate_devices().find(|device| {
            for extension in VulkanAllocator::optimal_device_extensions() {
                if !device.supports_extension(extension) {
                    return false;
                }
            }

            true
        }) {
            Some(physical) => (physical, VulkanAllocator::optimal_device_extensions()),

            None => {
                // Fallback to a device with the required extensions
                let physical = instance
                    .enumerate_devices()
                    .find(|device| {
                        for extension in VulkanAllocator::required_device_extensions() {
                            if !device.supports_extension(extension) {
                                return false;
                            }
                        }

                        true
                    })
                    .expect("no device");

                (physical, VulkanAllocator::required_device_extensions())
            }
        };

        let mut device_builder = Device::builder(&physical);

        for &extension in extensions {
            device_builder = device_builder.extension(extension);
        }

        let device = unsafe { device_builder.build(&instance) }.expect("device");
        let mut allocator = VulkanAllocator::new(&device).expect("allocator");

        assert!(allocator.is_format_supported(
            super::Format {
                code: super::Fourcc::Argb8888,
                modifier: super::Modifier::Linear,
            },
            ImageUsageFlags::SAMPLED
        ), "check failed");

        let image = allocator
            .create_buffer(100, 100, super::Fourcc::Argb8888, &[super::Modifier::Linear])
            .expect("create buffer");
        assert_eq!(image.width(), 100);
        assert_eq!(image.height(), 100);
        assert_eq!(image.format().code, super::Fourcc::Argb8888);
        assert_eq!(image.format().modifier, super::Modifier::Linear);
    }
}
