use crate::vulkan::error::VkError;

/// An error that may occur when creating a device.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// Device has no suitable queue family.
    ///
    /// Generally this means the device has no graphics capable queue family.
    #[error("device has no suitable queue family")]
    NoSuitableQueue,

    /// Vulkan API error.
    #[error(transparent)]
    Vk(#[from] VkError),
}
