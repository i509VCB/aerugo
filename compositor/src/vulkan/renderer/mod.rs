mod render_pass;

use std::{collections::HashSet, sync::Arc};

use smithay::{
    backend::{
        allocator::{self, dmabuf::Dmabuf},
        renderer::{Bind, Frame, ImportDma, ImportShm, Renderer, Texture, TextureFilter, Transform, Unbind},
    },
    reexports::wayland_server::protocol::wl_buffer,
    utils::{Buffer, Physical, Rectangle, Size},
    wayland::compositor::SurfaceData,
};

use super::{
    device::{Device, DeviceHandle},
    version::Version,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {}

#[derive(Debug)]
pub struct VulkanTexture {}

impl VulkanTexture {
    pub fn image(&self) -> &ash::vk::Image {
        todo!()
    }

    pub fn image_view(&self) -> &ash::vk::ImageView {
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

#[derive(Debug)]
pub struct VulkanFrame {}

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
        _damage: &[Rectangle<i32, Physical>],
        _src_transform: Transform,
        _alpha: f32,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn transformation(&self) -> Transform {
        todo!()
    }
}

#[derive(Debug)]
pub struct VulkanRenderer {
    /// The device handle.
    ///
    /// Since a vulkan renderer owns some vulkan objects, we need this handle to ensure objects do not outlive
    /// the renderer.
    device: Arc<DeviceHandle>,
}

impl VulkanRenderer {
    /// Returns a list of extensions the device enable to use a [`VulkanRenderer`].
    pub const fn required_extensions(version: Version) -> Result<&'static [&'static str], ()> {
        match version {
            Version::VERSION_1_0 => todo!(),
            Version::VERSION_1_1 => todo!(),
            Version::VERSION_1_2 => todo!(),

            _ => Err(()),
        }
    }

    // TODO: There may be some required device capabilities?

    pub fn new(device: &Device) -> Result<VulkanRenderer, ()> {
        // Verify the required extensions are supported.
        let version = device.version();

        if !Self::required_extensions(version)
            .expect("TODO Error type")
            .iter()
            .all(|extension| device.is_extension_enabled(extension))
        {
            todo!("Missing required extensions error")
        }

        todo!()
    }

    pub fn device(&self) -> Arc<DeviceHandle> {
        self.device.clone()
    }

    pub fn dmabuf_formats<'a>(&'a self) -> Box<dyn Iterator<Item = &'a allocator::Format> + 'a> {
        // We can lookup this information using `VkDrmFormatModifierPropertiesListEXT` extension to
        // `vkGetPhysicalDeviceFormatProperties2`
        todo!()
    }
}

impl Renderer for VulkanRenderer {
    type Error = Error;

    type TextureId = VulkanTexture;

    type Frame = VulkanFrame;

    fn downscale_filter(&mut self, _filter: TextureFilter) -> Result<(), Self::Error> {
        todo!()
    }

    fn upscale_filter(&mut self, _filter: TextureFilter) -> Result<(), Self::Error> {
        todo!()
    }

    fn render<F, R>(
        &mut self,
        _size: Size<i32, Physical>,
        _dst_transform: Transform,
        _rendering: F,
    ) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Self, &mut Self::Frame) -> R,
    {
        todo!()
    }
}

impl Bind<Dmabuf> for VulkanRenderer {
    fn bind(&mut self, _target: Dmabuf) -> Result<(), Self::Error> {
        todo!()
    }

    fn supported_formats(&self) -> Option<HashSet<allocator::Format>> {
        todo!()
    }
}

// TODO: Way to bind to a swapchain or possibly an arbitrary VkFrameBuffer?

impl Unbind for VulkanRenderer {
    fn unbind(&mut self) -> Result<(), Self::Error> {
        todo!()
    }
}

impl ImportDma for VulkanRenderer {
    fn import_dmabuf(&mut self, _dmabuf: &Dmabuf) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }
}

impl ImportShm for VulkanRenderer {
    fn import_shm_buffer(
        &mut self,
        _buffer: &wl_buffer::WlBuffer,
        _surface: Option<&SurfaceData>,
        _damage: &[Rectangle<i32, Buffer>],
    ) -> Result<Self::TextureId, Self::Error> {
        todo!()
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        // TODO
    }
}
