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

    fn clear(&mut self, _color: [f32; 4], _at: &[Rectangle<i32, Physical>]) -> Result<(), Self::Error> {
        todo!()
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
