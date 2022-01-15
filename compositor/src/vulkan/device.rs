use std::{
    fmt::{self, Formatter},
    sync::Arc,
};

use ash::vk::{DeviceCreateInfo, DevicePrivateDataCreateInfoEXT, DeviceQueueCreateInfo, ExtendsDeviceCreateInfo};

use super::{
    instance::{InstanceError, InstanceInner},
    physical_device::PhysicalDevice,
    queue::QueueFamily,
    Version,
};

/// A builder used to construct a device.
#[derive(Debug)]
pub struct DeviceBuilder<'i, 'p> {
    device: &'p PhysicalDevice<'i>,
    queues: Vec<QueueFamily>,
    enable_extensions: Vec<String>,
    features: Option<ash::vk::PhysicalDeviceFeatures>,
}

impl<'i, 'p> DeviceBuilder<'i, 'p> {
    /// Adds an instance extension to be requested when creating an [`Instance`].
    ///
    /// The extension must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the extension you are requesting is supported is to check if your extension is listed in
    /// [`Instance::enumerate_extensions`].
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.enable_extensions.push(extension.into());
        self
    }

    /// Indicates to Vulkan to create the queues for the queue family.
    ///
    /// In order for device creation to be successful, at least one queue must be created.
    pub fn create_queue(mut self, queue_family: &QueueFamily) -> Self {
        self.queues.push(*queue_family);
        self
    }

    /// The default features to enable when creating the device.
    pub fn features(mut self, features: ash::vk::PhysicalDeviceFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Returns a new device using the parameters passed into the builder.
    pub fn build(self) -> Result<Device, InstanceError> {
        // Use DevicePrivateDataCreateInfoEXT as a dummy generic for monomorphization.
        unsafe { self.build_impl::<DevicePrivateDataCreateInfoEXT>(None) }
    }

    /// Returns a new device using the parameters passed into the builder.
    ///
    /// This function also supports passing extension structs for additional information to be used when creating the
    /// device. To pass multiple extension structs, use the `push_next` function of the root extension struct.
    ///
    /// # Safety
    ///
    /// The extension struct must be in compliance with the Vulkan specification.
    pub unsafe fn build_with_extension<T: ExtendsDeviceCreateInfo>(
        self,
        extension: &mut T,
    ) -> Result<Device, InstanceError> {
        self.build_impl(Some(extension))
    }

    unsafe fn build_impl<E: ExtendsDeviceCreateInfo>(self, extension: Option<&mut E>) -> Result<Device, InstanceError> {
        if self.queues.is_empty() {
            todo!("Error, no queues")
        }

        let instance_handle = self.device.instance().handle();

        let queues = self
            .queues
            .iter()
            .map(|queue| {
                DeviceQueueCreateInfo {
                    queue_family_index: queue.index as u32,
                    queue_count: 1,
                    // TODO: Multi queue priorities?
                    p_queue_priorities: [1.0f32].as_ptr(),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        let mut create_info = DeviceCreateInfo::builder().queue_create_infos(&queues[..]);

        if let Some(extension) = extension {
            create_info = create_info.push_next(extension);
        }

        if let Some(features) = &self.features {
            create_info = create_info.enabled_features(features);
        }

        let device = instance_handle.create_device(self.device.handle(), &create_info, None)?;
        let inner = Arc::new(DeviceInner {
            instance: self.device.instance().0.clone(),
            device,
            version: self.device.version(),
            enabled_extensions: self.enable_extensions.clone(),
        });

        Ok(Device(inner))
    }
}

/// Represents a handle to an instantiated logical device.
#[derive(Debug)]
pub struct Device(Arc<DeviceInner>);

impl Device {
    /// Returns a new builder used to construct a [`Device`].
    pub fn builder<'i, 'p>(physical_device: &'p PhysicalDevice<'i>) -> DeviceBuilder<'i, 'p> {
        DeviceBuilder {
            device: physical_device,
            enable_extensions: vec![],
            queues: vec![],
            features: None,
        }
    }

    /// Returns maximum the version of the API the device supports.
    pub fn version(&self) -> Version {
        self.0.version
    }

    /// Returns a list of enabled extensions the device was created with.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.0.enabled_extensions.clone()
    }

    /// Returns true if one if the specified extension is enabled on the device.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn is_extension_enabled(&self, extension: &str) -> bool {
        self.0.enabled_extensions.iter().any(|name| name == extension)
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
        self.0.device.clone()
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

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) };
    }
}
