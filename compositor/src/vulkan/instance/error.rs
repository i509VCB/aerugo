use crate::vulkan::error::VkError;

/// An error that may occur when creating an instance.
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
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
