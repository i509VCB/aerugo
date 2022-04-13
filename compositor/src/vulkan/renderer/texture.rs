use ash::vk;
use smithay::backend::renderer::Texture;

#[derive(Debug)]
pub struct VulkanTexture();

impl VulkanTexture {
    pub fn memory(&self) -> &vk::DeviceMemory {
        todo!()
    }

    pub fn image(&self) -> &vk::Image {
        todo!()
    }

    pub fn image_view(&self) -> &vk::ImageView {
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
}
