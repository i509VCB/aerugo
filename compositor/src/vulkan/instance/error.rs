/// An error that may occur when using or creating an instance.
#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    /// The driver does not support the requested Vulkan API version.
    #[error("driver does not support the requested Vulkan API version")]
    IncompatibleDriver,

    /// The host or device has run out of memory.
    #[error("the host or device has run out of memory")]
    OutOfMemory,

    /// Some requested layers are not available.
    #[error("the following layers are not available: {}", .0.join(", "))]
    MissingLayers(Vec<String>),

    /// Some requested extensions are not available.
    #[error("the following extensions are not available: {}", .0.join(", "))]
    MissingExtensions(Vec<String>),

    /// Some other error occurred.
    #[error(transparent)]
    Other(ash::vk::Result),
}

impl From<ash::vk::Result> for InstanceError {
    fn from(err: ash::vk::Result) -> Self {
        match err {
            ash::vk::Result::ERROR_INCOMPATIBLE_DRIVER => InstanceError::IncompatibleDriver,
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => InstanceError::OutOfMemory,
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => InstanceError::OutOfMemory,
            err => InstanceError::Other(err),
        }
    }
}
