use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        renderer::{ExportDma, ImportDma, ImportDmaWl},
    },
    utils::{Buffer, Rectangle, Size},
};

use super::{DrmFormat, VulkanRenderer};

impl ImportDma for VulkanRenderer {
    fn import_dmabuf(
        &mut self,
        _dmabuf: &Dmabuf,
        _damage: Option<&[Rectangle<i32, Buffer>]>,
    ) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }

    fn dmabuf_formats<'a>(&'a self) -> Box<dyn Iterator<Item = &'a DrmFormat> + 'a> {
        Box::new(self.dmabuf_texture_formats())
    }
}

impl ImportDmaWl for VulkanRenderer {}

impl ExportDma for VulkanRenderer {
    fn export_framebuffer(&mut self, _size: Size<i32, Buffer>) -> Result<Dmabuf, Self::Error> {
        todo!()
    }

    fn export_texture(&mut self, _texture: &Self::TextureId) -> Result<Dmabuf, Self::Error> {
        todo!()
    }
}
