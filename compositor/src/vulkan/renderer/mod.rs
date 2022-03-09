mod format;
mod format_convert;
mod render_pass;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
pub struct VulkanFrame {
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    device: Arc<DeviceHandle>,
}

impl Frame for VulkanFrame {
    type Error = Error;

    type TextureId = VulkanTexture;

    fn clear(&mut self, color: [f32; 4], at: &[Rectangle<i32, Physical>]) -> Result<(), Self::Error> {
        // VUID-vkCmdClearAttachments-rectCount-arraylength
        if at.is_empty() {
            return Ok(());
        }

        // TODO: VUID-vkCmdClearAttachments-rect-02682 + VUID-vkCmdClearAttachments-rect-02683, extent w, h must be > 0.
        // TODO: VUID-vkCmdClearAttachments-pRects-00016, regions specified must be within render area of the render pass

        // TODO: What colorspace is float32 in?
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue { float32: color },
        };

        let attachments = [vk::ClearAttachment::builder()
            .clear_value(clear_value)
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            // VUID-vkCmdClearAttachments-aspectMask-02501
            .color_attachment(vk::ATTACHMENT_UNUSED)
            .build()];

        let rects = at
            .iter()
            .map(|rect| {
                let offset = vk::Offset2D::builder().x(rect.loc.x).y(rect.loc.y).build();

                let extent = vk::Extent2D::builder()
                    .width(rect.size.w as u32)
                    .height(rect.size.h as u32)
                    .build();

                let rect = vk::Rect2D::builder().offset(offset).extent(extent).build();

                vk::ClearRect::builder()
                    // VUID-vkCmdClearAttachments-layerCount-01934
                    .layer_count(1)
                    .rect(rect)
                    .build()
            })
            .collect::<Vec<_>>();

        unsafe {
            self.device
                .raw()
                .cmd_clear_attachments(self.command_buffer, &attachments[..], &rects[..])
        };

        Ok(())
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
    render_submit_fence: vk::Fence,

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
            render_submit_fence,
            target: None,
            // vert_shader: todo!(),
            // tex_frag_shader: todo!(),
            // quad_frag_shader: todo!(),
            pipelines: HashMap::new(),
            supports_dma,
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
        rendering: F,
    ) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Self, &mut Self::Frame) -> R,
    {
        if self.target.is_none() {
            return Err(Error::NoTargetFramebuffer);
        }

        let device = self.device.raw();

        // Vulkan requires a bound render target:
        // TODO

        // Enter a recording state
        let begin_info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::empty());

        unsafe { device.begin_command_buffer(self.render_command_buffer, &begin_info) }.map_err(VkError::from)?;

        let mut frame = VulkanFrame {
            command_buffer: self.render_command_buffer,
            render_pass: todo!(),
            device: self.device(),
        };

        // TODO: Set scissor box before invoking callback.

        let result = rendering(self, &mut frame);

        // Submit to queue.
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
        if let Some(target) = self.target.take() {
            unsafe {
                // TODO: VUID-vkDestroyFramebuffer-framebuffer-00892
                self.device.raw().destroy_framebuffer(target.framebuffer, None);
            }
        }

        Ok(())
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
            device.destroy_fence(self.render_submit_fence, None);
        }
    }
}

// Impl details

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
