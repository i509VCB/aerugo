//! Utilities for memory allocation in Vulkan.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ash::vk;

use crate::vulkan::error::VkError;

use super::{Error, VulkanRenderer};

// TODO: Move this to a module common to the allocator and renderer,
// TODO: It's probably quite useful to expose Allocation in public api.
pub(super) struct Allocator {
    // TODO: staging buffer utilities
    /// The current number of device allocations.
    allocation_count: Arc<AtomicUsize>,

    /// The maximum number of device allocations.
    ///
    /// Although the Vulkan specification states an implementation may return an error when the max allocation
    /// count is exceeded, it is still undefined behavior to exceed this value and the error code should not
    /// be used to indicate that.
    max_allocation_count: usize,
}

impl Allocator {
    /// Allocates some device memory.
    ///
    /// This function will return [`Err`] if the maximum number of allocations is reached.
    ///
    /// # Safety
    ///
    /// - The caller is responsible ensuring the valid usage requirements are met for allocating memory.
    /// - The caller is responsible for freeing the memory.
    pub unsafe fn allocate_memory(
        &self,
        device: &ash::Device,
        allocate_info: &vk::MemoryAllocateInfo,
    ) -> Result<(vk::DeviceMemory, AllocationId), Error> {
        let allocation = self.new_id()?;
        let memory = unsafe { device.allocate_memory(allocate_info, None) }.map_err(VkError::from)?;

        Ok((memory, allocation))
    }

    /// Creates a reference counted type which when dropped decreases the allocation count.
    ///
    /// This returns [`Err`] if the maximum number of allocations is exceeded.
    #[must_use = "Dropping the value decreases the allocation count"]
    pub fn new_id(&self) -> Result<AllocationId, Error> {
        let count = self.allocation_count.clone();

        count
            .fetch_update(Ordering::Release, Ordering::Relaxed, |count| {
                // Fail if the maximum allocation count would be exceeded.
                if self.max_allocation_count < count {
                    return None;
                }

                // Vulkan sets a hard limit of u32::MAX allocations that a device may track.
                count.checked_add(1)
            })
            .map_err(Error::TooManyAllocations)?;

        Ok(AllocationId(count))
    }
}

/// Reference counted type used to track the lifetime of an allocation.
#[derive(Debug)]
pub(super) struct AllocationId(Arc<AtomicUsize>);

impl Drop for AllocationId {
    fn drop(&mut self) {
        let result = self
            .0
            .fetch_update(Ordering::Release, Ordering::Relaxed, |count| count.checked_sub(1));

        // If there is underflow, it is likely some bug has occurred.
        debug_assert!(result.is_ok(), "device allocation count underflow",);
    }
}

impl VulkanRenderer {
    /// Returns the index of a memory type that supports the specified memory property flags.
    pub(super) fn get_memory_type_index(&self, required_bits: u32, flags: vk::MemoryPropertyFlags) -> Option<usize> {
        self.memory_properties
            .memory_types
            .iter()
            // Limit number of iterations to the number of memory types, as the rest are Default::default
            .take(self.memory_properties.memory_type_count as usize)
            .enumerate()
            .filter(|&(index, _)| required_bits & (1 << index) != 0)
            .map(|(_, ty)| ty)
            .position(|ty| ty.property_flags.contains(flags))
    }

    // TODO: Staging buffer utilities
    // TODO: Image creation
}
