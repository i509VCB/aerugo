use std::sync::Arc;

use ash::vk;
use smithay::{
    backend::renderer::Frame,
    utils::{Buffer, Physical, Rectangle, Transform},
};

use crate::vulkan::device::DeviceHandle;

use super::{texture::VulkanTexture, Error, RenderTarget};

#[derive(Debug)]
pub struct VulkanFrame {
    pub(super) command_buffer: vk::CommandBuffer,
    pub(super) full_clear_render_pass: vk::RenderPass,
    pub(super) partial_clear_clear_render_pass: vk::RenderPass,
    pub(super) target: RenderTarget,
    pub(super) started: bool,
    pub(super) device: Arc<DeviceHandle>,
}

impl Frame for VulkanFrame {
    type Error = Error;
    type TextureId = VulkanTexture;

    fn clear(&mut self, color: [f32; 4], at: &[Rectangle<i32, Physical>]) -> Result<(), Self::Error> {
        if at.is_empty() {
            // TODO: Should this succeed or fail?
            return Ok(());
        }

        // There are two ways to perform a clear in Vulkan:
        // 1. If we clear the entire viewport, we can use a render pass with a loadOp of CLEAR. Performing the
        //    clear at the start of the render pass is much more performant, but comes at the cost of needing
        //    to render everything else again.
        //
        // 2. If we cannot clear the entire viewport (the box(es) are smaller than the viewport), then we need
        //    to use the vkCmdClearAttachments command and a render pass with a loadOp of LOAD. Depending on
        //    the Vulkan implementation this may be less efficient, but we do not need to render everything
        //    again.
        //
        // To achieve the above, we always start the frame with no render pass set. The first command will set
        // the appropriate render pass.
        if !self.started {
            self.started = true;

            // Perform optimized clear on render pass start if we have a single at box which covers the whole
            // viewport.
            //
            // TODO: Better validation to determine whether a full clear should be done.
            if at.len() == 1
                && at[0].size.w as u32 == self.target.width
                && at[0].size.h as u32 == self.target.height
                && at[0].loc == (0, 0).into()
            {
                unsafe { self.full_clear(color) };
                return Ok(());
            }
        }

        todo!("damaged clear")
    }

    fn render_texture_from_to(
        &mut self,
        _texture: &Self::TextureId,
        _src: Rectangle<i32, Buffer>,
        _dst: Rectangle<f64, Physical>,
        _damage: &[Rectangle<i32, Buffer>],
        _src_transform: Transform,
        _alpha: f32,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn transformation(&self) -> Transform {
        todo!()
    }
}

impl VulkanFrame {
    /// Performs a full clear of the viewport.
    ///
    /// This will not preserve any existing content.
    ///
    /// # Safety
    ///
    /// The render pass must not already be begun (since this function starts a specific render pass).
    unsafe fn full_clear(&mut self, color: [f32; 4]) {
        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            // TODO: Check the cast, as a negative i32 value will be some huge value.
            extent: vk::Extent2D {
                width: self.target.width as u32,
                height: self.target.height as u32,
            },
        };

        let clear_values = &[vk::ClearValue {
            color: vk::ClearColorValue { float32: color },
        }];

        let begin_info = vk::RenderPassBeginInfo::builder()
            .framebuffer(self.target.framebuffer)
            .render_pass(self.full_clear_render_pass)
            .render_area(render_area)
            .clear_values(clear_values);

        unsafe {
            self.device
                .raw()
                .cmd_begin_render_pass(self.command_buffer, &begin_info, vk::SubpassContents::INLINE)
        };
    }
}
