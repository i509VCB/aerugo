use std::sync::Arc;

use smithay::backend::renderer::{Texture, TextureMapping};

use super::texture::TextureInner;

#[derive(Debug)]
pub struct VulkanMapping(Arc<TextureInner>);

impl Texture for VulkanMapping {
    fn width(&self) -> u32 {
        self.0.width
    }

    fn height(&self) -> u32 {
        self.0.height
    }
}

impl TextureMapping for VulkanMapping {
    fn flipped(&self) -> bool {
        todo!()
    }
}
