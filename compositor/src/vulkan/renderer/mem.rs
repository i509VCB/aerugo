use std::ptr;

use ash::vk;
use smithay::{
    backend::renderer::{ImportMem, ImportMemWl},
    reexports::wayland_server::protocol::{wl_buffer, wl_shm},
    utils::{Buffer, Rectangle, Size},
    wayland::compositor,
};

use crate::vulkan::{error::VkError, renderer::StagingBuffer};

use super::VulkanRenderer;

impl ImportMem for VulkanRenderer {
    fn import_memory(
        &mut self,
        data: &[u8],
        size: Size<i32, Buffer>,
        _flipped: bool,
    ) -> Result<Self::TextureId, Self::Error> {
        // Validate buffer parameters (*4 because of argb8888)
        if (size.w * size.h * 4) as usize > data.len() {
            todo!("err: invalid size")
        }

        let texture = unsafe { self.create_texture(vk::Format::B8G8R8A8_SRGB, (size.w as u32, size.h as u32).into()) }?;

        let device = self.device.raw();

        // Ensure we can create another memory allocation.
        let allocation_id = self.allocator.new_id()?;

        // Create the handle for the buffer and device memory first.
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(data.len() as u64);

        let buffer = unsafe { device.create_buffer(&buffer_create_info, None) }.map_err(VkError::from)?;
        let memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_type_index = match self.get_memory_type_index(
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        ) {
            Some(index) => index,
            None => unsafe {
                // Destroy the buffer handle to prevent leaking
                device.destroy_buffer(buffer, None);
                todo!("invalid memory type")
            },
        };

        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index as u32);

        let device_memory = match unsafe { device.allocate_memory(&memory_allocate_info, None) } {
            Ok(mem) => mem,
            Err(err) => unsafe {
                // Destroy the buffer handle to prevent leaking
                device.destroy_buffer(buffer, None);
                return Err(VkError::from(err).into());
            },
        };

        // Bind the buffer to the device memory to allow writing.
        if let Err(err) = unsafe { device.bind_buffer_memory(buffer, device_memory, 0) } {
            // Destroy the buffer handle and device memory to prevent leaking
            unsafe {
                device.destroy_buffer(buffer, None);
                device.free_memory(device_memory, None);
            }

            return Err(VkError::from(err).into());
        }

        // Map device memory to copy the data
        let mapped =
            match unsafe { device.map_memory(device_memory, 0, data.len() as u64, vk::MemoryMapFlags::empty()) } {
                Ok(mapped) => mapped,
                Err(err) => unsafe {
                    // Destroy the buffer handle and device memory to prevent leaking
                    device.destroy_buffer(buffer, None);
                    device.free_memory(device_memory, None);

                    return Err(VkError::from(err).into());
                },
            };

        unsafe {
            // TODO: Consider minMemoryMapAlignment when deciding if this is safe
            ptr::copy(data.as_ptr() as *const _, mapped, data.len());
            device.unmap_memory(device_memory);
        }

        // Record copy command into the command buffer
        self.staging_buffers.push(StagingBuffer {
            buffer,
            buffer_size: data.len() as u64,
            memory: device_memory,
            memory_allocation_id: allocation_id,
        });

        // Cleanup buffer when copy is complete

        Ok(texture)
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
