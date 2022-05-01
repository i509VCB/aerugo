use std::ffi::CStr;

use ash::{
    extensions::ext::PhysicalDeviceDrm,
    vk::{self, PhysicalDeviceDriverProperties, PhysicalDeviceProperties2},
};
use nix::sys::stat::makedev;
use smithay::backend::drm::DrmNode;

use super::{error::VkError, Version};

/// A physical device provided by a Vulkan instance.
#[derive(Debug)]
pub struct PhysicalDevice<'i> {
    pub(super) inner: &'i PhysicalDeviceInner,
}

impl PhysicalDevice<'_> {
    /// Returns the name of the device.
    pub fn name(&self) -> &str {
        &self.inner.device_name
    }

    /// Returns the highest version of the Vulkan API the physical device supports.
    ///
    /// This will be the lower of the physical device's maximum supported version and the specified version of
    /// the instance.
    pub fn version(&self) -> Version {
        self.inner.api_version
    }

    /// Returns a list of device extensions this device supports.
    pub fn supported_extensions(&self) -> Vec<String> {
        self.inner.extensions.clone()
    }

    /// Returns true if the device supports the specified device extension.
    pub fn supports_extension(&self, extension: &str) -> bool {
        self.inner.extensions.iter().any(|supported| supported == extension)
    }

    /// Returns info about the Vulkan driver.
    ///
    /// This may return [`None`] if the Vulkan instance did not enable the `VK_KHR_driver_properties`
    /// instance extension.
    pub fn driver(&self) -> Option<DriverInfo> {
        self.inner.driver_info.clone()
    }

    /// Returns the type of the device.
    pub fn ty(&self) -> vk::PhysicalDeviceType {
        self.inner.ty
    }

    /// Returns the features the device supports.
    ///
    /// Checking if any additional features are supported may be done using [`ash::vk::PhysicalDeviceFeatures2`].  
    pub fn features(&self) -> ash::vk::PhysicalDeviceFeatures {
        self.inner.features
    }

    pub fn primary_node(&self) -> Option<DrmNode> {
        self.inner.primary_node
    }

    pub fn render_node(&self) -> Option<DrmNode> {
        self.inner.render_node
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
        self.inner.phy
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

#[derive(Debug)]
pub(super) struct PhysicalDeviceInner {
    pub(super) phy: vk::PhysicalDevice,
    pub(super) extensions: Vec<String>,
    pub(super) device_name: String,
    pub(super) api_version: Version,
    pub(super) ty: vk::PhysicalDeviceType,
    pub(super) driver_info: Option<DriverInfo>,
    pub(super) limits: vk::PhysicalDeviceLimits,
    pub(super) features: vk::PhysicalDeviceFeatures,
    pub(super) render_node: Option<DrmNode>,
    pub(super) primary_node: Option<DrmNode>,
}

impl PhysicalDeviceInner {
    pub fn new(instance: &ash::Instance, phy: vk::PhysicalDevice) -> Result<PhysicalDeviceInner, VkError> {
        let extensions = unsafe { instance.enumerate_device_extension_properties(phy) }?
            .iter()
            .map(|extension| {
                let name = unsafe { CStr::from_ptr(&extension.extension_name as *const _) };
                name.to_string_lossy().to_string()
            })
            .collect::<Vec<_>>();

        let features = unsafe { instance.get_physical_device_features(phy) };
        let properties = unsafe { instance.get_physical_device_properties(phy) };

        let device_name = unsafe { CStr::from_ptr(&properties.device_name as *const _) };
        let device_name = device_name.to_string_lossy().to_string();
        let api_version = Version::from_raw(properties.api_version);
        let ty = properties.device_type;
        let limits = properties.limits;

        // This extension was promoted to core in version 1.2
        let driver_info = if extensions.iter().any(|e| e == "VK_KHR_driver_properties") {
            let mut driver_properties = PhysicalDeviceDriverProperties::default();
            let mut properties = PhysicalDeviceProperties2::builder().push_next(&mut driver_properties);

            // SAFETY: Extension is available
            unsafe { instance.get_physical_device_properties2(phy, &mut properties) };

            let driver_name = unsafe { CStr::from_ptr(&driver_properties.driver_name as *const _) };
            let driver_info = unsafe { CStr::from_ptr(&driver_properties.driver_info as *const _) };
            let driver_name = driver_name.to_string_lossy().to_string();
            let driver_info = driver_info.to_string_lossy().to_string();

            Some(DriverInfo {
                id: driver_properties.driver_id,
                name: driver_name,
                info: driver_info,
                conformance: driver_properties.conformance_version,
            })
        } else {
            None
        };

        let (primary_node, render_node) = if extensions.iter().any(|ext| ext == "VK_EXT_physical_device_drm") {
            // SAFETY: Physical device supports the VK_EXT_physical_device_drm extension.
            let drm_properties = unsafe { PhysicalDeviceDrm::get_properties(instance, phy) };

            let primary = if drm_properties.has_primary == vk::TRUE {
                DrmNode::from_dev_id(makedev(
                    drm_properties.primary_major as u64,
                    drm_properties.primary_minor as u64,
                ))
                .ok()
            } else {
                None
            };

            let render = if drm_properties.has_render == vk::TRUE {
                DrmNode::from_dev_id(makedev(
                    drm_properties.render_major as u64,
                    drm_properties.render_minor as u64,
                ))
                .ok()
            } else {
                None
            };

            (primary, render)
        } else {
            (None, None)
        };

        Ok(PhysicalDeviceInner {
            phy,
            extensions,
            device_name,
            api_version,
            ty,
            driver_info,
            limits,
            features,
            render_node,
            primary_node,
        })
    }
}
