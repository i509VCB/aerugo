use std::collections::HashSet;

use ash::vk;
use smithay::backend::renderer::{Bind, Texture, Unbind};

use crate::vulkan::error::VkError;

use super::{texture::VulkanTexture, DrmFormat, RenderTarget, VulkanRenderer};

impl Bind<VulkanTexture> for VulkanRenderer {
    fn bind(&mut self, target: VulkanTexture) -> Result<(), Self::Error> {
        self.unbind()?;

        let render_pass = self.renderpasses.get(&target.format()).copied();
        let render_pass = match render_pass {
            Some(pass) => pass,
            None => unsafe { self.create_renderpass(target.format()) }?,
        };

        let attachments = [target.image_view()];
        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(target.width())
            .height(target.height())
            .layers(1);
        let framebuffer =
            unsafe { self.device().raw().create_framebuffer(&framebuffer_create_info, None) }.map_err(VkError::from)?;

        self.target = Some(RenderTarget {
            framebuffer,
            render_pass,
            width: target.width(),
            height: target.height(),
        });

        Ok(())
    }

    fn supported_formats(&self) -> Option<HashSet<DrmFormat>> {
        todo!()
    }
}

// TODO: Swapchain images.

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
