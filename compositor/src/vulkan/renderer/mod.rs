mod format;
mod format_convert;
mod render_pass;

use std::{collections::HashSet, sync::Arc};

use ash::vk::{self, CommandBufferAllocateInfo, CommandPoolCreateInfo};
use smithay::{
    backend::{
        allocator::{self, dmabuf::Dmabuf},
        renderer::{Bind, Frame, ImportDma, ImportShm, Renderer, Texture, TextureFilter, Transform, Unbind},
    },
    reexports::wayland_server::protocol::{wl_buffer, wl_shm},
    utils::{Buffer, Physical, Rectangle, Size},
    wayland::compositor::SurfaceData,
};

use super::{
    device::{Device, DeviceHandle},
    error::VkError,
    version::Version,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Vk(#[from] VkError),
}

#[derive(Debug)]
pub struct VulkanTexture {}

impl VulkanTexture {
    pub fn image(&self) -> &ash::vk::Image {
        todo!()
    }

    pub fn image_view(&self) -> &ash::vk::ImageView {
        todo!()
    }
}

impl Texture for VulkanTexture {
    fn width(&self) -> u32 {
        todo!()
    }

    fn height(&self) -> u32 {
        todo!()
    }

    fn size(&self) -> Size<i32, Buffer> {
        todo!()
    }
}

#[derive(Debug)]
pub struct VulkanFrame {}

impl Frame for VulkanFrame {
    type Error = Error;

    type TextureId = VulkanTexture;

    fn clear(&mut self, _color: [f32; 4], _at: &[Rectangle<i32, Physical>]) -> Result<(), Self::Error> {
        todo!()
    }

    fn render_texture_from_to(
        &mut self,
        _texture: &Self::TextureId,
        _src: Rectangle<i32, Buffer>,
        _dst: Rectangle<f64, Physical>,
        _damage: &[Rectangle<i32, Physical>],
        _src_transform: Transform,
        _alpha: f32,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn transformation(&self) -> Transform {
        todo!()
    }
}

#[derive(Debug)]
pub struct VulkanRenderer {
    command_buffer: vk::CommandBuffer,
    command_pool: vk::CommandPool,
    /// The device handle.
    ///
    /// Since a vulkan renderer owns some vulkan objects, we need this handle to ensure objects do not outlive
    /// the renderer.
    device: Arc<DeviceHandle>,
}

impl VulkanRenderer {
    /// Returns a list of device extensions the device must enable to use a [`VulkanRenderer`].
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub fn required_device_extensions(version: Version) -> Result<&'static [&'static str], ()> {
        if version >= Version::VERSION_1_2 {
            Ok(&[
                "VK_KHR_external_memory_fd",
                "VK_EXT_external_memory_dma_buf",
                "VK_EXT_image_drm_format_modifier",
            ])
        } else if version >= Version::VERSION_1_1 {
            Ok(&[
                "VK_KHR_external_memory_fd",
                "VK_EXT_external_memory_dma_buf",
                "VK_EXT_image_drm_format_modifier",
                // Promoted in Vulkan 1.2, enabled here to satisfy VUID-vkCreateDevice-ppEnabledExtensionNames-01387.
                "VK_KHR_image_format_list",
            ])
        } else {
            Err(())
        }
    }

    // TODO: There may be some required device capabilities?

    pub fn new(device: &Device) -> Result<VulkanRenderer, Error> {
        // Verify the required extensions are supported.
        let version = device.version();

        // VUID-vkCreateDevice-ppEnabledExtensionNames-01387
        if !Self::required_device_extensions(version)
            .expect("TODO Error type no version")
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            todo!("Missing required extensions error")
        }

        // Create the command pool for Vulkan
        let pool_info = CommandPoolCreateInfo::builder().queue_family_index(device.queue_family_index() as u32);

        let device = device.handle();
        let raw_device = device.raw();

        let command_pool = unsafe { raw_device.create_command_pool(&pool_info, None) }.map_err(VkError::from)?;

        // Create the command buffers
        let command_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        let command_buffer = unsafe { raw_device.allocate_command_buffers(&command_buffer_info) }
            .map_err(VkError::from)?
            .into_iter()
            .next()
            .unwrap();

        let mut renderer = VulkanRenderer {
            command_buffer,
            command_pool,
            device,
        };

        // Check which formats the renderer supports
        renderer.load_formats()?;

        Ok(renderer)
    }

    pub fn device(&self) -> Arc<DeviceHandle> {
        self.device.clone()
    }
}

impl Renderer for VulkanRenderer {
    type Error = Error;

    type TextureId = VulkanTexture;

    type Frame = VulkanFrame;

    fn downscale_filter(&mut self, _filter: TextureFilter) -> Result<(), Self::Error> {
        todo!()
    }

    fn upscale_filter(&mut self, _filter: TextureFilter) -> Result<(), Self::Error> {
        todo!()
    }

    fn render<F, R>(
        &mut self,
        _size: Size<i32, Physical>,
        _dst_transform: Transform,
        _rendering: F,
    ) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Self, &mut Self::Frame) -> R,
    {
        todo!()
    }
}

impl Bind<Dmabuf> for VulkanRenderer {
    fn bind(&mut self, _target: Dmabuf) -> Result<(), Self::Error> {
        todo!()
    }

    fn supported_formats(&self) -> Option<HashSet<allocator::Format>> {
        todo!()
    }
}

// TODO: Way to bind to a swapchain or possibly an arbitrary VkFrameBuffer?

impl Unbind for VulkanRenderer {
    fn unbind(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl ImportDma for VulkanRenderer {
    fn import_dmabuf(&mut self, _dmabuf: &Dmabuf) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }

    fn dmabuf_formats<'a>(&'a self) -> Box<dyn Iterator<Item = &'a allocator::Format> + 'a> {
        todo!()
    }
}

impl ImportShm for VulkanRenderer {
    fn import_shm_buffer(
        &mut self,
        _buffer: &wl_buffer::WlBuffer,
        _surface: Option<&SurfaceData>,
        _damage: &[Rectangle<i32, Buffer>],
    ) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }

    fn shm_formats(&self) -> &[wl_shm::Format] {
        todo!()
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        let raw = self.device.raw();

        // Destruction of objects must happen in the opposite order they are created.
        unsafe {
            // Command buffers are created by a command pool.
            raw.free_command_buffers(self.command_pool, &[self.command_buffer]);
            raw.destroy_command_pool(self.command_pool, None);
        }

        // Finally, we let the implicit drop of `Arc<DeviceHandle>` free the device if no other handles exist.
    }
}

/// Returns properties about the specified image format.
///
/// If [`None`] is returned, then the device does not support the specified image format.
///
/// # Safety
///
/// Per the valid usage requirement `VUID-VkPhysicalDeviceImageFormatInfo2-usage-requiredbitmask`, the usage
/// must be specified.
///
/// If `drm_modifier_info` is [`Some`], then the device must support the `VK_EXT_image_drm_format_modifier`
/// extension.
unsafe fn get_image_format_properties(
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    drm_modifier_info: Option<&mut vk::PhysicalDeviceImageDrmFormatModifierInfoEXT>,
) -> Result<Option<vk::ImageFormatProperties>, Error> {
    // instance.get_physical_device_image_format_properties2

    let mut format_info = vk::PhysicalDeviceImageFormatInfo2::builder()
        .ty(vk::ImageType::TYPE_2D)
        .format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage);

    let mut image_format_properties = vk::ImageFormatProperties2::builder();

    if let Some(drm_modifier_info) = drm_modifier_info {
        format_info = format_info.push_next(drm_modifier_info);
    }

    if let Err(result) = unsafe {
        instance.get_physical_device_image_format_properties2(physical, &format_info, &mut image_format_properties)
    } {
        return if result != vk::Result::ERROR_FORMAT_NOT_SUPPORTED {
            Err(Error::Vk(VkError::from(result)))
        } else {
            // Unsupported format
            Ok(None)
        };
    }

    let format_properties = image_format_properties.image_format_properties;

    Ok(Some(format_properties))
}
