mod dma;
mod format;
mod mem;

pub mod bind;
pub mod frame;
pub mod mapping;
pub mod texture;

use std::{collections::HashMap, sync::Arc};

use ash::vk::{self, CommandBufferAllocateInfo, CommandPoolCreateInfo};
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
    version::Version,
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

    #[error("the required wl_shm formats are not supported")]
    MissingRequiredFormats,
}

#[derive(Debug)]
pub struct VulkanRenderer {
    /// Command pool used to allocate the staging and rendering command buffers.
    command_pool: vk::CommandPool,

    /// Staging command buffer.
    ///
    /// This command buffer is used when textures need to be uploaded to the GPU.
    staging_command_buffer: vk::CommandBuffer,

    /// Whether the staging command buffer is recording commands.
    recording_staging_buffer: bool,

    /// Rendering command buffer.
    ///
    /// The render command buffer has a dependency on the staging command buffer, meaning any texture imports
    /// will always complete before the render command buffer starts executing commands.
    render_command_buffer: vk::CommandBuffer,

    /// Fence used to signal all submitted command buffers have completed execution.
    submit_fence: vk::Fence,

    /// Currently bound render target.
    ///
    /// Rendering will fail if the render target is not set.
    target: Option<RenderTarget>,

    /// All the graphics pipelines created by the renderer.
    ///
    /// Each render pass may only have attachments of the matching format, so we need to construct a
    /// renderpass and by proxy a pipeline for each format.
    pipelines: HashMap<vk::Format, GraphicsPipeline>,

    // Shaders
    // vert_shader: vk::ShaderModule,
    // tex_frag_shader: vk::ShaderModule,
    // quad_frag_shader: vk::ShaderModule,
    /// Information about the supported shm formats, such as the max extent of an image.
    shm_format_info: Vec<ShmFormatInfo>,

    /// Supported shm formats.
    shm_formats: Vec<wl_shm::Format>,

    /// Supported render formats for a dmabuf.
    dma_render_formats: Vec<DrmFormat>,

    /// Supported texture formats for a dmabuf.
    dma_texture_formats: Vec<DrmFormat>,

    /// Whether this renderer may import or export some [`Dmabuf`].
    ///
    /// This is only true if the following extensions are enabled on the device:
    /// * `VK_KHR_external_memory_fd`
    /// * `VK_EXT_external_memory_dma_buf`
    ///
    /// If this is false, all dmabuf import and export functions will fail.
    supports_dma: bool,

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
    pub fn optimal_device_extensions(version: Version) -> Result<&'static [&'static str], UnsupportedVulkanVersion> {
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
            Err(UnsupportedVulkanVersion)
        }
    }

    /// Returns a list of the device extensions the device must enable to use a [`VulkanRenderer`].
    ///
    /// This extension list contains the absolute minimum requirements for the renderer. Note that a renderer
    /// constructed from a device with these extensions enabled will be unable to use a [`Dmabuf`] for import
    /// or export.
    ///
    /// This list satisfies the requirement that all enabled extensions also enable their dependencies
    /// (`VUID-vkCreateDevice-ppEnabledExtensionNames-01387`).
    pub fn required_device_extensions(version: Version) -> Result<&'static [&'static str], UnsupportedVulkanVersion> {
        if version >= Version::VERSION_1_2 {
            Ok(&["VK_EXT_image_drm_format_modifier"])
        } else if version >= Version::VERSION_1_1 {
            Ok(&[
                "VK_EXT_image_drm_format_modifier",
                // Promoted in Vulkan 1.2, enabled here to satisfy VUID-vkCreateDevice-ppEnabledExtensionNames-01387.
                "VK_KHR_image_format_list",
            ])
        } else {
            Err(UnsupportedVulkanVersion)
        }
    }

    // TODO: There may be some required device capabilities?

    pub fn new(device: &Device) -> Result<VulkanRenderer, Error> {
        // Verify the required extensions are supported.
        let version = device.version();

        // VUID-vkCreateDevice-ppEnabledExtensionNames-01387
        if !Self::required_device_extensions(version)?
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            return Err(Error::MissingRequiredExtensions);
        }

        // Test if the renderer supports Dmabuf external memory.
        let supports_dma = Self::optimal_device_extensions(version)
            .unwrap()
            .iter()
            .all(|extension| device.is_extension_enabled(extension));

        let device = device.handle();
        let raw_device = device.raw();

        // TODO: Shaders and etc

        // Create the command pool for Vulkan
        let command_pool_info = CommandPoolCreateInfo::builder().queue_family_index(device.queue_family_index() as u32);
        let command_pool =
            unsafe { raw_device.create_command_pool(&command_pool_info, None) }.map_err(VkError::from)?;

        let fence_create_info = vk::FenceCreateInfo::builder();
        let render_submit_fence =
            unsafe { raw_device.create_fence(&fence_create_info, None) }.map_err(VkError::from)?;

        // Render command buffer
        let render_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let render_command_buffer = unsafe { raw_device.allocate_command_buffers(&render_buffer_info) }
            .map_err(VkError::from)?
            .into_iter()
            .next()
            .unwrap();

        // Staging command buffer
        let staging_buffer_info = CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let staging_command_buffer = unsafe { raw_device.allocate_command_buffers(&staging_buffer_info) }
            .map_err(VkError::from)?
            .into_iter()
            .next()
            .unwrap();

        let mut renderer = VulkanRenderer {
            command_pool,
            staging_command_buffer,
            recording_staging_buffer: false,
            render_command_buffer,
            submit_fence: render_submit_fence,
            target: None,
            pipelines: HashMap::new(),
            shm_format_info: Vec::new(),
            shm_formats: Vec::new(),
            supports_dma,
            device,
            dma_render_formats: Vec::new(),
            dma_texture_formats: Vec::new(),
            // vert_shader: todo!(),
            // tex_frag_shader: todo!(),
            // quad_frag_shader: todo!(),
        };

        // Check which formats the renderer supports
        renderer.load_formats()?;

        Ok(renderer)
    }

    pub fn dmabuf_render_formats(&self) -> impl Iterator<Item = &'_ DrmFormat> {
        self.dma_render_formats.iter()
    }

    pub fn dmabuf_texture_formats(&self) -> impl Iterator<Item = &'_ DrmFormat> {
        self.dma_texture_formats.iter()
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

    fn id(&self) -> usize {
        todo!()
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        let device = self.device.raw();

        unsafe {
            for pipeline in self.pipelines.values() {
                device.destroy_pipeline(pipeline.quad, None);
                device.destroy_pipeline(pipeline.texture, None);
                // TODO: VUID-vkDestroyRenderPass-renderPass-00873
                device.destroy_render_pass(pipeline.render_pass, None);
            }

            // Command buffers must be freed before the command pool.
            device.free_command_buffers(
                self.command_pool,
                &[self.staging_command_buffer, self.render_command_buffer],
            );
            device.destroy_command_pool(self.command_pool, None);

            // VUID-vkDestroyFence-fence-01120: All queue submission commands for fence have completed since the fence
            // must be signalled before exiting the rendering functions.
            device.destroy_fence(self.submit_fence, None);
        }
    }
}

// Impl details

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

#[derive(Debug)]
struct GraphicsPipeline {
    format: vk::Format,
    render_pass: vk::RenderPass,
    texture: vk::Pipeline,
    quad: vk::Pipeline,
}

impl VulkanRenderer {
    unsafe fn bind_framebuffer(
        &mut self,
        render_pass: vk::RenderPass,
        attachment: vk::ImageView,
        w: u32,
        h: u32,
    ) -> Result<(), VkError> {
        let attachment = &[attachment];

        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(attachment)
            .width(w)
            .height(h)
            .layers(1);

        let framebuffer = unsafe { self.device.raw().create_framebuffer(&create_info, None) }.map_err(VkError::from)?;

        self.target = Some(RenderTarget {
            framebuffer,
            width: w,
            height: h,
        });

        Ok(())
    }
}
