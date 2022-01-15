use std::ffi::CStr;

use ash::extensions::ext::PhysicalDeviceDrm;
use smithay::backend::drm::{DrmNode, NodeType};

use super::{
    instance::{Instance, InstanceError},
    queue::QueueFamily,
    Version,
};

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
    queue_families: Vec<QueueFamily>,
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

                let queue_families = unsafe { instance_handle.get_physical_device_queue_family_properties(device) }
                    .iter()
                    .enumerate()
                    .map(|(index, properties)| QueueFamily {
                        inner: *properties,
                        index,
                    })
                    .collect::<Vec<_>>();

                Ok(PhysicalDevice {
                    instance,
                    inner: device,

                    // Some pre fetched fields that are useful during enumeration
                    name,
                    properties,
                    features,
                    extensions,
                    queue_families,
                })
            })
            .collect::<Result<Vec<_>, InstanceError>>()?
            .into_iter())
    }

    // TODO: Add DRM feature attribute in smithay

    /// Enumerates over the available physical devices provided by the instance, selecting the device which corresponds
    /// to the DRM node.
    ///
    /// This function will only find the desired device if the device supports the [VK_EXT_physical_device_drm]
    /// extension.
    ///
    /// [VK_EXT_physical_device_drm]: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkPhysicalDeviceDrmPropertiesEXT.html
    pub fn with_drm_node(
        instance: &Instance,
        node: impl AsRef<DrmNode>,
    ) -> Result<Option<PhysicalDevice<'_>>, InstanceError> {
        Ok(PhysicalDevice::enumerate(instance)?.find(|device| {
            let handle = unsafe { device.handle() };

            if device.supports_extension("VK_EXT_physical_device_drm") {
                let node = node.as_ref();

                // SAFETY: Physical device supports for VK_EXT_physical_device_drm
                let drm_properties = unsafe { PhysicalDeviceDrm::get_properties(&instance.handle(), handle) };

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
    pub fn version(&self) -> Version {
        Version::from_raw(self.properties.api_version)
    }

    /// Returns a list of extensions this device supports.
    pub fn supported_extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }

    /// Returns true if the device supports the specified extension.
    pub fn supports_extension(&self, extension: &str) -> bool {
        self.extensions.iter().any(|supported| supported == extension)
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

    /// Returns an iterator over the queue families of the device.
    pub fn queue_families(&self) -> impl Iterator<Item = QueueFamily> + '_ {
        self.queue_families.iter().copied()
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
