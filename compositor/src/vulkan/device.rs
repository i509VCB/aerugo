use std::{
    fmt::{self, Formatter},
    sync::Arc,
};

use super::{InstanceInner, Version};

// TODO: Do we want to consider being able to put user data on the device handle?

/// Represents a handle to an instantiated logical device.
#[derive(Debug)]
pub struct Device {
    inner: Arc<DeviceInner>,
}

impl Device {
    /// Returns the version of the API the device has been created with.
    pub fn version(&self) -> Version {
        self.inner.version
    }

    /// Returns a list of enabled extensions the device was created with.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.inner.enabled_extensions.clone()
    }

    /// Returns true if one if the specified extension is enabled on the device.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn is_extension_enabled(&self, extension: &str) -> bool {
        self.inner.enabled_extensions.iter().any(|name| name == extension)
    }

    /// Returns a raw handle to the underlying [`ash::Device`].
    ///
    /// The returned handle may be used to access portions of the Vulkan API not in scope of the abstractions in this
    /// module.
    ///
    /// # Safety
    /// - The device must not be destroyed.
    /// - The caller must guarantee usage of the handle and any objects created using the device do not exceed the
    /// lifetime of the device.
    pub unsafe fn handle(&self) -> ash::Device {
        self.inner.device.clone()
    }
}

pub(crate) struct DeviceInner {
    instance: Arc<InstanceInner>,
    device: ash::Device,
    version: Version,
    enabled_extensions: Vec<String>,
}

impl fmt::Debug for DeviceInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DeviceInner").field(&self.device.handle()).finish()
    }
}
