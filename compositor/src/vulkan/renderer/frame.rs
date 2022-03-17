use std::sync::Arc;

use ash::vk;
use smithay::{
    backend::renderer::Frame,
    utils::{Buffer, Physical, Rectangle, Transform},
};

use crate::vulkan::device::DeviceHandle;

use super::{texture::VulkanTexture, Error};

#[derive(Debug)]
pub struct VulkanFrame {
    pub(super) command_buffer: vk::CommandBuffer,
    pub(super) render_pass: vk::RenderPass,
    pub(super) device: Arc<DeviceHandle>,
}

impl Frame for VulkanFrame {
    type Error = Error;

    type TextureId = VulkanTexture;

    fn clear(&mut self, color: [f32; 4], at: &[Rectangle<i32, Physical>]) -> Result<(), Self::Error> {
        /*
          We could use a load/store op during a render pass to clear, but that does not support specifying damage.
          Instead this function uses vkCmdClearAttachments to support damage boxes.
        */

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
