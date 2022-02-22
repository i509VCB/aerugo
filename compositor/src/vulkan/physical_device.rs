use std::ffi::CStr;

use ash::{
    extensions::ext::PhysicalDeviceDrm,
    vk::{self, PhysicalDeviceDriverProperties, PhysicalDeviceProperties2},
};
use smithay::backend::drm::{DrmNode, NodeType};

use super::{
    error::VkError,
    instance::{Instance, InstanceError},
    Version,
};

/// A physical device provided by a Vulkan instance.
#[derive(Debug)]
pub struct PhysicalDevice<'i> {
    instance: &'i Instance,
    inner: ash::vk::PhysicalDevice,

    /* Some pre fetched fields that are useful during enumeration */
    name: String,
    driver: Option<DriverInfo>,
    properties: ash::vk::PhysicalDeviceProperties,
    features: ash::vk::PhysicalDeviceFeatures,
    extensions: Vec<String>,
}

impl PhysicalDevice<'_> {
    /// Enumerates over the physical devices
    pub fn enumerate(instance: &Instance) -> Result<impl Iterator<Item = PhysicalDevice<'_>>, VkError> {
        // SAFETY: The lifetime on PhysicalDevice ensures the Physical devices created using the handle do not
        // outlive the instance.
        let raw_instance = unsafe { instance.raw() };

        Ok(unsafe { raw_instance.enumerate_physical_devices() }?
            .into_iter()
            .map(|device| {
                let features = unsafe { raw_instance.get_physical_device_features(device) };

                let extensions = unsafe { raw_instance.enumerate_device_extension_properties(device) }?
                    .iter()
                    .map(|extension| {
                        let name = unsafe { CStr::from_ptr(&extension.extension_name as *const _) };
                        name.to_str()
                            .expect("Invalid UTF-8 in Vulkan extension name")
                            .to_owned()
                    })
                    .collect::<Vec<_>>();

                let properties = unsafe { raw_instance.get_physical_device_properties(device) };

                let name = unsafe { CStr::from_ptr(&properties.device_name as *const _) }
                    .to_str()
                    .expect("Invalid UTF-8 in Vulkan extension name")
                    .to_owned();

                let supports_driver_info = {
                    // Promoted to core in 1.2, so all implementations must have it.
                    let mut supported = instance.version() >= Version::VERSION_1_2;
                    // Otherwise the instance must have enabled `VK_KHR_get_physical_device_properties2`,
                    // which has been promoted to core in 1.1
                    supported |= extensions.iter().any(|e| e == "VK_KHR_driver_properties");
                    supported
                };

                // Promoted to core in >= 1.2
                let driver = if supports_driver_info {
                    let mut driver_properties = PhysicalDeviceDriverProperties::default();
                    let mut properties = PhysicalDeviceProperties2::builder().push_next(&mut driver_properties);

                    // SAFETY: The Vulkan version is high enough or the required extensions are supported.
                    unsafe { raw_instance.get_physical_device_properties2(device, &mut properties) };

                    // SAFETY: Vulkan specification guarantees both strings are null-terminated UTF-8
                    let driver_name = unsafe { CStr::from_ptr(&driver_properties.driver_name as *const _) };
                    let driver_info = unsafe { CStr::from_ptr(&driver_properties.driver_info as *const _) };
                    let driver_name = driver_name.to_str().unwrap().to_owned();
                    let driver_info = driver_info.to_str().unwrap().to_owned();

                    Some(DriverInfo {
                        id: driver_properties.driver_id,
                        name: driver_name,
                        info: driver_info,
                        conformance: driver_properties.conformance_version,
                    })
                } else {
                    None
                };

                Ok(PhysicalDevice {
                    instance,
                    inner: device,

                    // Some pre fetched fields that are useful during enumeration
                    name,
                    driver,
                    properties,
                    features,
                    extensions,
                })
            })
            .collect::<Result<Vec<_>, VkError>>()?
            .into_iter())
    }

    // TODO: Add DRM feature attribute in smithay

    /// Enumerates over the available physical devices provided by the instance, selecting the device which corresponds
    /// to the DRM node.
    ///
    /// This function will only find the desired device if the device supports the [`VK_EXT_physical_device_drm`]
    /// extension.
    ///
    /// [`VK_EXT_physical_device_drm`]: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkPhysicalDeviceDrmPropertiesEXT.html
    pub fn with_drm_node(
        instance: &Instance,
        node: impl AsRef<DrmNode>,
    ) -> Result<Option<PhysicalDevice<'_>>, InstanceError> {
        Ok(PhysicalDevice::enumerate(instance)?.find(|device| {
            let handle = unsafe { device.handle() };

            if device.supports_extension("VK_EXT_physical_device_drm") {
                let node = node.as_ref();

                // SAFETY: Physical device supports the VK_EXT_physical_device_drm extension.
                let drm_properties = unsafe { PhysicalDeviceDrm::get_properties(instance.raw(), handle) };

                match node.ty() {
                    NodeType::Primary if drm_properties.has_primary == ash::vk::TRUE => {
                        drm_properties.primary_major as u64 == node.major()
                            && drm_properties.primary_minor as u64 == node.minor()
                    }

                    NodeType::Render if drm_properties.has_render == ash::vk::TRUE => {
                        drm_properties.render_major as u64 == node.major()
                            && drm_properties.render_minor as u64 == node.minor()
                    }

                    _ => false,
                }
            } else {
                false
            }
        }))
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
    ///
    /// This will be the lower of the physical device's maximum supported version and the specified version of
    /// the instance.
    pub fn version(&self) -> Version {
        Version::from_raw(self.properties.api_version)
    }

    /// Returns a list of device extensions this device supports.
    pub fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    /// Returns true if the device supports the specified device extension.
    pub fn supports_extension(&self, extension: &str) -> bool {
        self.extensions.iter().any(|supported| supported == extension)
    }

    /// Returns info about the Vulkan driver.
    ///
    /// This may return [`None`] if the Vulkan instance has not enabled the `VK_KHR_driver_properties`
    /// instance extension.
    pub fn driver(&self) -> Option<DriverInfo> {
        self.driver.clone()
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
    /// The returned handle may be used to access portions of the Vulkan API not in scope of the abstractions
    /// in this module.
    ///
    /// The data provided using this handle is only correct as long as the parent instance is alive.
    ///
    /// # Safety
    /// - The instance must not be destroyed.
    /// - The caller must guarantee the handle does not to outlive the instance (since the physical device is
    /// immediately invalid once the instance is destroyed).
    pub unsafe fn handle(&self) -> ash::vk::PhysicalDevice {
        self.inner
    }
}

/// Description of a Vulkan driver.
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// ID which identifies the driver.
    pub id: vk::DriverId,

    /// The name of the driver.
    pub name: String,

    /// Information describing the driver.
    ///
    /// This may include information such as the version.
    pub info: String,

    /// The Vulkan conformance test this driver is conformant against.
    pub conformance: vk::ConformanceVersion,
}
