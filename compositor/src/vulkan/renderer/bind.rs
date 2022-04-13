use std::collections::HashSet;

use smithay::backend::{
    renderer::{Bind, Unbind},
};

use super::{texture::VulkanTexture, DrmFormat, VulkanRenderer};

impl Bind<VulkanTexture> for VulkanRenderer {
    fn bind(&mut self, _target: VulkanTexture) -> Result<(), Self::Error> {
        todo!()
    }

    fn supported_formats(&self) -> Option<HashSet<DrmFormat>> {
        todo!()
    }
}

// TODO: Swapchain image.

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
