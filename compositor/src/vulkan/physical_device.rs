#![allow(dead_code)] // Because this is an experiment for a future pull request.
#![warn(missing_docs)]

use std::ffi::CStr;

use super::{Instance, InstanceError, Version};

/// A physical device provided by a Vulkan instance.
#[derive(Debug)]
pub struct PhysicalDevice<'i> {
    instance: &'i Instance,
    inner: ash::vk::PhysicalDevice,
    /* Some pre fetched fields that are useful during enumeration */
    name: String,
    properties: ash::vk::PhysicalDeviceProperties,
    features: ash::vk::PhysicalDeviceFeatures,
    extensions: Vec<String>,
}

impl PhysicalDevice<'_> {
    /// Enumerates over the physical devices
    pub fn enumerate(instance: &Instance) -> Result<impl Iterator<Item = PhysicalDevice<'_>>, InstanceError> {
        // SAFETY: Instance lifetime on PhysicalDevice ensures the Physical devices created using the handle and child
        // objects do not outlive the instance.
        Ok(unsafe { instance.handle().enumerate_physical_devices() }?
            .into_iter()
            .map(|device| {
                let instance_handle = unsafe { instance.handle() };
                let features = unsafe { instance_handle.get_physical_device_features(device) };

                let extensions = unsafe { instance_handle.enumerate_device_extension_properties(device) }?
                    .iter()
                    .map(|extension| {
                        let name = unsafe { CStr::from_ptr(&extension.extension_name as *const _) };
                        name.to_str()
                            .expect("Invalid UTF-8 in Vulkan extension name")
                            .to_owned()
                    })
                    .collect();

                let properties = unsafe { instance_handle.get_physical_device_properties(device) };

                let name = unsafe { CStr::from_ptr(&properties.device_name as *const _) }
                    .to_str()
                    .expect("Invalid UTF-8 in Vulkan extension name")
                    .to_owned();

                Ok(PhysicalDevice {
                    instance,
                    inner: device,
                    /* Some pre fetched fields that are useful during enumeration */
                    name,
                    properties,
                    features,
                    extensions,
                })
            })
            .collect::<Result<Vec<_>, InstanceError>>()?
            .into_iter())
    }

    /// Returns the instance the physical device belongs to.
    pub fn instance(&self) -> &Instance {
        self.instance
    }

    /// Returns the name of the device.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the highest version of the Vulkan API the physical device supports.
    pub fn version(&self) -> Version {
        Version::from_raw(self.properties.api_version)
    }

    /// Returns a list of extensions this device supports.
    pub fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    /// Returns some properties about the physical device.
    pub fn properties(&self) -> ash::vk::PhysicalDeviceProperties {
        self.properties
    }

    /// Returns the features the device supports.
    ///
    /// Checking if any additional features are supported may be done using [`ash::vk::PhysicalDeviceFeatures2`].  
    pub fn features(&self) -> ash::vk::PhysicalDeviceFeatures {
        self.features
    }

    /// Returns a raw handle to the underlying [`ash::vk::PhysicalDevice`].
    ///
    /// The returned handle may be used to access portions of the Vulkan API not in scope of the abstractions in this
    /// module.
    ///
    /// # Safety
    /// - The instance must not be destroyed.
    /// - The caller must guarantee usage of the handle and any objects created using the physical device do not exceed
    /// the lifetime which owns this physical device..
    pub unsafe fn handle(&self) -> ash::vk::PhysicalDevice {
        self.inner
    }
}
