use std::{
    fmt::{self, Formatter},
    sync::Arc,
};

use ash::vk::{DeviceCreateInfo, DevicePrivateDataCreateInfoEXT, DeviceQueueCreateInfo, ExtendsDeviceCreateInfo};

use super::{
    instance::{InstanceError, InstanceHandle},
    physical_device::PhysicalDevice,
    queue::QueueFamily,
    Version,
};

pub struct DeviceHandle {
    instance: Arc<InstanceHandle>,
    device: ash::Device,
    version: Version,
    enabled_extensions: Vec<String>,
}

impl DeviceHandle {
    /// Returns a reference to the underlying [`ash::Device`].
    ///
    /// # Safety
    /// - Callers must NOT destroy the returned device.
    /// - Child objects created using the device must not outlive the device.
    ///
    /// These safety requirements may be checked by enabling validation layers.
    pub unsafe fn raw(&self) -> &ash::Device {
        &self.device
    }
}

impl fmt::Debug for DeviceHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceInner")
            .field("version", &self.version)
            .field("enabled_extensions", &self.enabled_extensions)
            .finish_non_exhaustive()
    }
}

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        // TODO: The Vulkan specification suggests applications can use `vkDeviceWaitIdle` to ensure no work
        // is active on the device before destroying it. Although this may block.

        // SAFETY: The Vulkan specification states the following requirements:
        //
        // > All child objects created using device must have been destroyed prior to destroying device.
        // This first requirement is met since accessing the handle is unsafe, and callers must guarantee no
        // child objects outlive the device.
        //
        // > Host access to device must be externally synchronized.
        // Host access is externally synchronized since the DeviceHandle is given to users inside an Arc.
        unsafe { self.device.destroy_device(None) };
    }
}

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
        // SAFETY: Caller guaranteed the extension structs are compliant.
        unsafe { self.build_impl(Some(extension)) }
    }

    unsafe fn build_impl<E: ExtendsDeviceCreateInfo>(self, extension: Option<&mut E>) -> Result<Device, InstanceError> {
        if self.queues.is_empty() {
            todo!("Error, no queues")
        }

        let instance_handle = self.device.instance().handle();
        // SAFETY: The Arc<InstanceHandle> stored in the device guarantees the device will not outlive the
        // instance.
        let raw_instance = unsafe { instance_handle.raw() };

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

        // SAFETY: The Arc<InstanceHandle> stored in the device guarantees the device will not outlive the
        // instance.
        let device = unsafe { raw_instance.create_device(self.device.handle(), &create_info, None) }?;
        let inner = Arc::new(DeviceHandle {
            instance: instance_handle,
            device,
            version: self.device.version(),
            enabled_extensions: self.enable_extensions.clone(),
        });

        Ok(Device(inner))
    }
}

/// Represents a handle to an instantiated logical device.
#[derive(Debug)]
pub struct Device(Arc<DeviceHandle>);

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

    // TODO: Make this the created version, not max supported.
    /// Returns maximum the version of the API the device supports.
    pub fn version(&self) -> Version {
        self.0.version
    }

    /// Returns a list of the device's enabled extensions.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.0.enabled_extensions.clone()
    }

    /// Returns true if the specified extension is enabled for the device.
    ///
    /// Some parts of the Vulkan API may only be used if the corresponding extension is enabled on the device.
    pub fn is_extension_enabled(&self, extension: &str) -> bool {
        self.0.enabled_extensions.iter().any(|name| name == extension)
    }

    /// Returns a raw handle to the underlying [`ash::Device`].
    ///
    /// The Vulkan API enforces a strict lifetimes over objects that are created, meaning child objects
    /// cannot outlive their device. A great way to ensure the device will live long enough is storing a
    /// handle inside the container of child objects.
    ///
    /// Since a device is a child object of a [`Instance`], storing a handle also ensures child objects do not
    /// outlive the instance.
    pub fn handle(&self) -> Arc<DeviceHandle> {
        self.0.clone()
    }
}
