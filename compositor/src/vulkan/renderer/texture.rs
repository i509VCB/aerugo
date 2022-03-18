use std::sync::Arc;

use ash::vk;
use smithay::backend::renderer::Texture;

#[derive(Debug)]
pub struct VulkanTexture(Arc<TextureInner>);

impl VulkanTexture {
    pub fn memory(&self) -> &vk::DeviceMemory {
        &self.0.memory
    }

    pub fn image(&self) -> &vk::Image {
        &self.0.image
    }

    pub fn image_view(&self) -> &vk::ImageView {
        &self.0.image_view
    }
}

impl Texture for VulkanTexture {
    fn width(&self) -> u32 {
        self.0.width
    }

    fn height(&self) -> u32 {
        self.0.height
    }
}

#[derive(Debug)]
pub(super) struct TextureInner {
    pub(super) memory: vk::DeviceMemory,
    pub(super) image: vk::Image,
    pub(super) image_view: vk::ImageView,
    pub(super) width: u32,
    pub(super) height: u32,
}
