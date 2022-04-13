mod bind;
mod format;
mod mem;

pub mod frame;
pub mod texture;

use std::sync::Arc;

use ash::vk;
use smithay::{
    backend::{
        allocator::Format as DrmFormat,
        renderer::{Renderer, TextureFilter},
    },
    reexports::wayland_server::protocol::wl_shm,
    utils::{Physical, Size, Transform},
};

use self::{frame::VulkanFrame, texture::VulkanTexture};

use super::{
    device::{Device, DeviceHandle},
    error::VkError,
    UnsupportedVulkanVersion,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Vk(#[from] VkError),

    #[error(transparent)]
    Version(#[from] UnsupportedVulkanVersion),

    #[error("required extensions are not enabled")]
    MissingRequiredExtensions,

    #[error("no target framebuffer to render to, to bind a framebuffer, use `VulkanRenderer::bind`")]
    NoTargetFramebuffer,

    #[error("required extensions for dmabuf import/export are not enabled or available")]
    DmabufNotSupported,

    #[error("the required wl_shm formats are not supported")]
    MissingRequiredFormats,
}

/// TODO:
/// - ExportMem
/// - ImportDma
/// - Bind<Dmabuf>
/// - ExportDma
#[derive(Debug)]
pub struct VulkanRenderer {
    /// Command pool used to allocate the staging and rendering command buffers.
    command_pool: vk::CommandPool,

    /// Renderer format info.
    formats: Formats,

    /// Currently bound render target.
    ///
    /// Rendering will fail if the render target is not set.
    target: Option<RenderTarget>,

    /// The device handle.
    ///
    /// Since a Vulkan renderer owns some Vulkan objects, we need this handle to ensure objects do not outlive
    /// the renderer.
    device: Arc<DeviceHandle>,
}

impl VulkanRenderer {
    /// Returns a list of device extensions the device must enable to use a [`VulkanRenderer`] most optimally.
    ///
    /// This set of extensions is required in order to use a [`Dmabuf`] for import or export into the renderer.
    ///
    /// If the device does not support all of the specified extensions, a smaller extension subset in
    /// [`VulkanRenderer::required_device_extensions`] may be used instead.
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub fn optimal_device_extensions() -> &'static [&'static str] {
        &[
            "VK_KHR_external_memory_fd",
            "VK_EXT_external_memory_dma_buf",
            "VK_EXT_image_drm_format_modifier",
            // Or Vulkan 1.2
            "VK_KHR_image_format_list",
        ]
    }

    /// Returns a list of the device extensions the device must enable to use a [`VulkanRenderer`].
    ///
    /// This extension list contains the absolute minimum requirements for the renderer. Note that a renderer
    /// constructed from a device with these extensions enabled will be unable to use a [`Dmabuf`] for import
    /// or export.
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub fn required_device_extensions() -> &'static [&'static str] {
        &[
            "VK_EXT_image_drm_format_modifier",
            // Or Vulkan 1.2
            "VK_KHR_image_format_list",
        ]
    }

    // TODO: There may be some required device capabilities?

    pub fn new(device: &Device) -> Result<VulkanRenderer, Error> {
        // Verify the required extensions are supported.
        // VUID-vkCreateDevice-ppEnabledExtensionNames-01387
        if !Self::required_device_extensions()
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            return Err(Error::MissingRequiredExtensions);
        }

        let queue_family_index = device.queue_family_index() as u32;
        let device = device.handle();

        // Create the renderer with everything filled in using null handles.
        //
        // The reason for initializing everything with null handles is to allow the drop code of the renderer
        // to properly destroy every object, meaning no memory is leaked if part of the initialization process
        // fails.
        //
        // Vulkan explicitly allows passing null handles into destruction functions, which do nothing.
        let mut renderer = VulkanRenderer {
            command_pool: vk::CommandPool::null(),
            formats: Formats {
                shm_format_info: Vec::new(),
                shm_formats: Vec::new(),
                dma_render_formats: Vec::new(),
                dma_texture_formats: Vec::new(),
            },
            target: None,
            device,
        };

        let device_handle = renderer.device();
        let device_handle = device_handle.raw();

        let command_pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family_index);
        renderer.command_pool =
            unsafe { device_handle.create_command_pool(&command_pool_info, None) }.map_err(VkError::from)?;

        // Check which formats the renderer supports
        renderer.load_formats()?;

        // It's extremely likely we will need to import a buffer in one of the mandatory shm formats, so
        // initialize the A/Xrgb8888 pipelines now.
        // TODO

        Ok(renderer)
    }

    pub fn dmabuf_render_formats(&self) -> impl Iterator<Item = &'_ DrmFormat> {
        self.formats.dma_render_formats.iter()
    }

    pub fn dmabuf_texture_formats(&self) -> impl Iterator<Item = &'_ DrmFormat> {
        self.formats.dma_texture_formats.iter()
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
        todo!("not implemented")
    }

    fn upscale_filter(&mut self, _filter: TextureFilter) -> Result<(), Self::Error> {
        todo!("not implemented")
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

    fn id(&self) -> usize {
        todo!("not implemented")
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        let device = self.device.raw();

        unsafe {
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}

// Impl details

#[derive(Debug)]
struct Formats {
    /// Information about the supported shm formats, such as the max extent of an image.
    shm_format_info: Vec<ShmFormatInfo>,

    /// Supported shm formats.
    shm_formats: Vec<wl_shm::Format>,

    /// Supported render formats for a dmabuf.
    ///
    /// This is the list of formats that may be rendered to a dmabuf.
    dma_render_formats: Vec<DrmFormat>,

    /// Supported texture formats for a dmabuf.
    ///
    /// This is the list of formats that may be sampled from a dmabuf.
    dma_texture_formats: Vec<DrmFormat>,
}

#[derive(Debug)]
struct ShmFormatInfo {
    shm: wl_shm::Format,
    vk: vk::Format,
    max_extent: vk::Extent2D,
}

#[derive(Debug)]
struct RenderTarget {
    framebuffer: vk::Framebuffer,
    width: u32,
    height: u32,
}
