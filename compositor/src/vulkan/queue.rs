/// A family of queues on a device.
///
/// Each family of queues has a varying number of queues and capabilities.
#[derive(Debug)]
pub struct QueueFamily {
    pub(crate) inner: ash::vk::QueueFamilyProperties,
    pub(crate) index: usize,
}

impl QueueFamily {
    /// Returns flags which represent the capabilities of the queues in the queue family.
    pub fn flags(&self) -> ash::vk::QueueFlags {
        self.inner.queue_flags
    }

    /// Returns the number of queues available.
    pub fn queue_count(&self) -> u32 {
        self.inner.queue_count
    }

    /// Returns a handle to the raw ash type representing the properties of a queue family.
    pub fn inner(&self) -> &ash::vk::QueueFamilyProperties {
        &self.inner
    }
}
