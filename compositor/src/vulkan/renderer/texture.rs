use std::sync::Arc;

use ash::vk;
use smithay::{
    backend::{allocator::dmabuf::MAX_PLANES, renderer::Texture},
    utils::{Buffer as BufferCoord, Size},
};

use crate::vulkan::{device::DeviceHandle, error::VkError};

use super::{alloc::AllocationId, Error, VulkanRenderer};

#[derive(Debug)]
pub struct VulkanTexture(TextureInner);

impl VulkanTexture {
    /// The device memory associated with the texture.
    ///
    /// The first entry in the array will always contain some device memory. Depending on the type of image,
    /// namely if the image was imported from a dmabuf, the other three planes may contain memory or a null
    /// handle.
    pub fn memory(&self) -> [vk::DeviceMemory; MAX_PLANES] {
        self.0.memory
    }

    pub fn image(&self) -> vk::Image {
        self.0.image
    }

    pub fn image_view(&self) -> vk::ImageView {
        self.0.image_view
    }
}

impl Texture for VulkanTexture {
    fn width(&self) -> u32 {
        self.0.size.w
    }

    fn height(&self) -> u32 {
        self.0.size.h
    }
}

#[derive(Debug)]
pub(super) struct TextureInner {
    size: Size<u32, BufferCoord>,
    memory: [vk::DeviceMemory; MAX_PLANES],
    image: vk::Image,
    image_view: vk::ImageView,
    // The first entry is the id associated with `memory[0]`.
    allocation_ids: (AllocationId, [Option<AllocationId>; 3]),
    device_handle: Arc<DeviceHandle>,
}

impl Drop for TextureInner {
    fn drop(&mut self) {
        let device = self.device_handle.raw();

        unsafe {
            device.destroy_image_view(self.image_view, None);
            device.destroy_image(self.image, None);

            for memory in self.memory {
                device.free_memory(memory, None);
            }
        }
    }
}

impl VulkanRenderer {
    pub unsafe fn create_texture(
        &self,
        format: vk::Format,
        size: Size<u32, BufferCoord>,
    ) -> Result<VulkanTexture, Error> {
        // TODO: Max extent

        // Make sure we can create more device memory.
        let allocation_id = self.allocator.new_id()?;
        let device = self.device.raw();
        let image_create_info = vk::ImageCreateInfo::builder()
            .format(format)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            // TODO: Supporting specific modifiers will require changes
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
            .extent(vk::Extent3D {
                width: size.w,
                height: size.h,
                depth: 1,
            })
            .image_type(vk::ImageType::TYPE_2D);

        let mut inner = TextureInner {
            size,
            memory: [vk::DeviceMemory::null(); MAX_PLANES],
            image: vk::Image::null(),
            image_view: vk::ImageView::null(),
            allocation_ids: (allocation_id, [None, None, None]),
            device_handle: self.device(),
        };

        inner.image = unsafe { device.create_image(&image_create_info, None) }.map_err(VkError::from)?;

        // Allocate memory for the image
        let memory_requirements = unsafe { device.get_image_memory_requirements(inner.image) };

        let memory_type_index = self
            .get_memory_type_index(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("TODO: Handle no memory type");

        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index as u32);

        inner.memory[0] = unsafe { device.allocate_memory(&memory_allocate_info, None) }.map_err(VkError::from)?;
        unsafe { device.bind_image_memory(inner.image, inner.memory[0], 0) }.map_err(VkError::from)?;

        // Create the image view.
        let components = vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            // TODO: Will vary depending on the format, todo: DRM info needed
            a: vk::ComponentSwizzle::IDENTITY,
        };

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1)
            .build();

        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(components)
            .subresource_range(subresource_range)
            .image(inner.image);

        inner.image_view = unsafe { device.create_image_view(&image_view_create_info, None) }.map_err(VkError::from)?;

        Ok(VulkanTexture(inner))
    }
}
