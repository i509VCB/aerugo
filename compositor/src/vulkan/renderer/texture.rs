use ash::vk;
use smithay::{
    backend::renderer::Texture,
    utils::{Buffer, Size},
};

#[derive(Debug)]
pub struct VulkanTexture {}

impl VulkanTexture {
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

    fn size(&self) -> Size<i32, Buffer> {
        todo!()
    }
}
