mod format;
mod render_pass;

use std::{collections::HashSet, sync::Arc};

use ash::vk::{self, CommandBufferAllocateInfo, CommandPoolCreateInfo, DrmFormatModifierPropertiesListEXT};
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
    /// Format and modifier pairs the renderer may import a dmabuf using.
    dma_formats: HashSet<allocator::Format>,
    /// Formats the renderer may import shared memory using.
    shm_formats: Vec<wl_shm::Format>,
}

impl VulkanRenderer {
    /// Returns a list of device extensions the device must enable to use a [`VulkanRenderer`].
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
                // Promoted in Vulkan 1.2
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
        let raw_device = unsafe { device.raw() };

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

        // Build the list of valid dmabuf and shm formats
        let mut shm_formats = vec![];
        let dma_formats = {
            let instance = unsafe { device.instance.raw() };
            let mut dma_formats = vec![];

            for code in format::formats() {
                if let Some((vk, _)) = format::fourcc_to_vk(code) {
                    // First we need to query how many entries are available.
                    let mut formats_ext = DrmFormatModifierPropertiesListEXT::builder();
                    let mut properties2 = vk::FormatProperties2::builder().push_next(&mut formats_ext);

                    // SAFETY: VK_EXT_image_drm_format_modifier is enabled
                    // Null pointer for pDrmFormatModifierProperties is safe when obtaining count.
                    unsafe {
                        instance.get_physical_device_format_properties2(device.physical, vk, &mut properties2);
                    }

                    // Immediately end the mutable borrow on `formats_ext` in order to ensure the formats_ext is accessible.
                    drop(properties2);

                    let modifier_count = formats_ext.drm_format_modifier_count as usize;
                    let mut modifiers = Vec::with_capacity(modifier_count);
                    formats_ext = formats_ext.drm_format_modifier_properties(&mut modifiers[..]);

                    // Now we can create a new struct since the fields are filled in on the extension.
                    let mut properties2 = vk::FormatProperties2::builder().push_next(&mut formats_ext);

                    // Initialize the value
                    unsafe {
                        instance.get_physical_device_format_properties2(device.physical, vk, &mut properties2);

                        // SAFETY: Elements from 0..len() were just initialized.
                        modifiers.set_len(modifier_count);
                    }

                    // If a format has some number of modifiers, then we can import wl_shm buffers for the
                    // format.
                    if !modifiers.is_empty() {
                        if let Some(format) = format::fourcc_to_wl(code) {
                            shm_formats.push(format);
                        }
                    }

                    for modifier_properties in modifiers {
                        dma_formats.push(allocator::Format {
                            code,
                            modifier: allocator::Modifier::from(modifier_properties.drm_format_modifier),
                        })
                    }
                }
            }

            HashSet::from_iter(dma_formats.into_iter())
        };

        // Ensure the shm renderer contains the mandatory formats
        if !shm_formats.iter().any(|format| format == &wl_shm::Format::Argb8888) {
            todo!("Missing argb8888")
        }

        // Ensure the shm renderer contains the mandatory formats
        if !shm_formats.iter().any(|format| format == &wl_shm::Format::Xrgb8888) {
            todo!("Missing xrgb8888")
        }

        Ok(VulkanRenderer {
            command_buffer,
            command_pool,
            device,
            dma_formats,
            shm_formats,
        })
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
        Some(self.dma_formats.clone())
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
        Box::new(self.dma_formats.iter())
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
        &self.shm_formats[..]
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        let raw = unsafe { self.device.raw() };

        // Destruction of objects must happen in the opposite order they are created.
        unsafe {
            // Command buffers are created by a command pool.
            raw.free_command_buffers(self.command_pool, &[self.command_buffer]);
            raw.destroy_command_pool(self.command_pool, None);
        }

        // Finally, we let the implicit drop of `Arc<DeviceHandle>` free the device if no other handles exist.
    }
}
