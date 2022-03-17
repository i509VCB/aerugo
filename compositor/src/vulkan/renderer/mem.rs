use smithay::{
    backend::renderer::{ExportMem, ImportMem, ImportMemWl},
    reexports::wayland_server::protocol::{wl_buffer, wl_shm},
    utils::{Buffer, Rectangle, Size},
    wayland::compositor,
};

use super::{mapping::VulkanMapping, VulkanRenderer};

impl ImportMem for VulkanRenderer {
    fn import_memory(
        &mut self,
        _data: &[u8],
        _size: Size<i32, Buffer>,
        _flipped: bool,
    ) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }

    fn update_memory(
        &mut self,
        _texture: &Self::TextureId,
        _data: &[u8],
        _region: Rectangle<i32, Buffer>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

impl ImportMemWl for VulkanRenderer {
    fn import_shm_buffer(
        &mut self,
        _buffer: &wl_buffer::WlBuffer,
        _surface: Option<&compositor::SurfaceData>,
        _damage: &[Rectangle<i32, Buffer>],
    ) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }

    fn shm_formats(&self) -> &[wl_shm::Format] {
        &self.shm_formats[..]
    }
}

impl ExportMem for VulkanRenderer {
    type TextureMapping = VulkanMapping;

    fn copy_framebuffer(&mut self, _region: Rectangle<i32, Buffer>) -> Result<Self::TextureMapping, Self::Error> {
        todo!()
    }

    fn copy_texture(
        &mut self,
        _texture: &Self::TextureId,
        _region: Rectangle<i32, Buffer>,
    ) -> Result<Self::TextureMapping, Self::Error> {
        todo!()
    }

    fn map_texture<'a>(&mut self, _texture_mapping: &'a Self::TextureMapping) -> Result<&'a [u8], Self::Error> {
        todo!()
    }
}
