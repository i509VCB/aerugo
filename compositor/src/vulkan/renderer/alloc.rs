//! Utilities to allocate memory for Vulkan

use ash::vk;

use super::VulkanRenderer;

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
