use crate::vulkan::{error::VkError, version::Version};

/// An error that may occur when creating an instance.
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error("smithay requires vulkan 1.1, you requested version {0}")]
    UnsupportedVulkanVersion(Version),

    /// Some requested layers are not available.
    #[error("the following layers are not available: {}", .0.join(", "))]
    MissingLayers(Vec<String>),

    /// Some requested extensions are not available.
    #[error("the following extensions are not available: {}", .0.join(", "))]
    MissingExtensions(Vec<String>),

    /// Vulkan API error.
    #[error(transparent)]
    Vk(#[from] VkError),
}
