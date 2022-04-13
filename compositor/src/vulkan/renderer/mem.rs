use smithay::{
    backend::renderer::{ImportMem, ImportMemWl},
    reexports::wayland_server::protocol::{wl_buffer, wl_shm},
    utils::{Buffer, Rectangle, Size},
    wayland::compositor,
};

use super::VulkanRenderer;

impl ImportMem for VulkanRenderer {
    fn import_memory(
        &mut self,
        _data: &[u8],
        _size: Size<i32, Buffer>,
        _flipped: bool,
    ) -> Result<Self::TextureId, Self::Error> {
        // Create staging buffer - TODO: Util to create buffer
        // Map memory to the buffer
        // Create image
        // Allocate device memory for image
        // Record copy command into the command buffer
        // Cleanup buffer when copy is complete

        todo!()
    }

    fn update_memory(
        &mut self,
        _texture: &Self::TextureId,
        _data: &[u8],
        _region: Rectangle<i32, Buffer>,
    ) -> Result<(), Self::Error> {
        // Create staging buffer - TODO: Util to create buffer
        // Map memory to the buffer
        // Perform copy command to update the memory

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
        // See import_memory, just with more formats

        todo!()
    }

    fn shm_formats(&self) -> &[wl_shm::Format] {
        &self.formats.shm_formats[..]
    }
}
