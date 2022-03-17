use smithay::backend::renderer::{Texture, TextureMapping};

#[derive(Debug)]
pub struct VulkanMapping {}

impl Texture for VulkanMapping {
    fn width(&self) -> u32 {
        todo!()
    }

    fn height(&self) -> u32 {
        todo!()
    }
}

impl TextureMapping for VulkanMapping {
    fn flipped(&self) -> bool {
        todo!()
    }
}
