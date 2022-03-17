use std::collections::HashSet;

use smithay::backend::{
    allocator::dmabuf::Dmabuf,
    renderer::{Bind, Unbind},
};

use super::{DrmFormat, VulkanRenderer};

impl Bind<Dmabuf> for VulkanRenderer {
    fn bind(&mut self, _target: Dmabuf) -> Result<(), Self::Error> {
        todo!()
    }

    fn supported_formats(&self) -> Option<HashSet<DrmFormat>> {
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
