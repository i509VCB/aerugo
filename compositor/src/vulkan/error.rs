use std::fmt;

use ash::vk;

/// Type representing a Vulkan API error.
#[derive(Debug, thiserror::Error)]
pub struct VkError {
    #[source]
    err: vk::Result,
    kind: ErrorKind,
}

impl VkError {
    pub fn error(&self) -> vk::Result {
        self.err
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for VkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::LayerNotPresent
            | ErrorKind::ExtensionNotPresent
            | ErrorKind::InitializationFailed
            | ErrorKind::IncompatibleDriver
            | ErrorKind::HostOutOfMemory
            | ErrorKind::DeviceOutOfMemory
            | ErrorKind::DeviceLost
            | ErrorKind::TooManyObjects
            | ErrorKind::Implementation => {
                write!(f, "{} (code: {})", &self.kind, self.err.as_raw())
            }

            ErrorKind::Other => write!(f, "error code: {}", self.err.as_raw()),
        }
    }
}

/// A kind of error that may occur when using the Vulkan APIs.
///
/// The variants represent possible errors that cannot be proven at compile time or while validation layers
/// are enabled.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum ErrorKind {
    /// A required layer is not present.
    #[error("a specified layer is not present")]
    LayerNotPresent,

    /// A required extension is not present.
    #[error("a requested extension is not present")]
    ExtensionNotPresent,

    /// Initializing a Vulkan object has failed.
    #[error("initializing a vulkan object has failed")]
    InitializationFailed,

    /// Unable to find a Vulkan driver.
    #[error("unable to find a vulkan driver")]
    IncompatibleDriver,

    /// The host has run out of memory.
    #[error("the host has run out of memory")]
    HostOutOfMemory,

    /// The device has run out of memory.
    #[error("the device has run out of memory")]
    DeviceOutOfMemory,

    /// The device has been lost.
    #[error("the device has been lost")]
    DeviceLost,

    /// Too many objects of some type have been already created.
    #[error("too many objects of some type have been already created")]
    TooManyObjects,

    /// Unknown error in application or Vulkan implementation
    #[error("unknown error in application or vulkan implementation")]
    Implementation,

    /// Other vulkan error.
    #[error("other vulkan error")]
    Other,
}

impl From<vk::Result> for VkError {
    fn from(err: vk::Result) -> Self {
        let kind = match err {
            vk::Result::ERROR_LAYER_NOT_PRESENT => ErrorKind::LayerNotPresent,
            vk::Result::ERROR_EXTENSION_NOT_PRESENT => ErrorKind::ExtensionNotPresent,
            vk::Result::ERROR_INITIALIZATION_FAILED => ErrorKind::InitializationFailed,
            vk::Result::ERROR_INCOMPATIBLE_DRIVER => ErrorKind::IncompatibleDriver,
            vk::Result::ERROR_OUT_OF_HOST_MEMORY => ErrorKind::HostOutOfMemory,
            vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => ErrorKind::DeviceOutOfMemory,
            vk::Result::ERROR_DEVICE_LOST => ErrorKind::DeviceLost,
            vk::Result::ERROR_UNKNOWN => ErrorKind::Implementation,

            _ => ErrorKind::Other,
        };

        VkError { err, kind }
    }
}
