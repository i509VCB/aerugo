mod error;

use std::{
    ffi::CString,
    fmt::{self, Formatter},
    sync::Arc,
};

use ash::vk::{
    DeviceCreateInfo, DevicePrivateDataCreateInfoEXT, DeviceQueueCreateInfo, ExtendsDeviceCreateInfo, QueueFlags,
};

use super::{error::VkError, instance::InstanceHandle, physical_device::PhysicalDevice, Version};

pub use self::error::*;

pub struct DeviceHandle {
    device: ash::Device,
    pub(crate) physical: ash::vk::PhysicalDevice,
    queue_family_index: usize,
    queue: ash::vk::Queue,
    version: Version,
    enabled_extensions: Vec<String>,
    pub(crate) instance: Arc<InstanceHandle>,
}

impl DeviceHandle {
    /// Returns a reference to the underlying [`ash::Device`].
    ///
    /// # Safety
    /// - Callers must NOT destroy the returned device.
    /// - Child objects created using the device must not outlive the device
    /// (`VUID-vkDestroyDevice-device-00378`).
    ///
    /// These safety requirements may be checked by enabling validation layers.
    pub unsafe fn raw(&self) -> &ash::Device {
        &self.device
    }

    pub unsafe fn queue(&self) -> &ash::vk::Queue {
        &self.queue
    }

    pub fn queue_family_index(&self) -> usize {
        self.queue_family_index
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
        // > VUID-vkDestroyDevice-device-00378: All child objects created using device must have been
        // > destroyed prior to destroying device. Access to the raw handle of the devices is unsafe unless
        // > the caller upholds the lifetime requirements.
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

    /// The default features to enable when creating the device.
    pub fn features(mut self, features: ash::vk::PhysicalDeviceFeatures) -> Self {
        self.features = Some(features);
        self
    }

    /// Returns a new device using the parameters passed into the builder.
    pub fn build(self) -> Result<Device, DeviceError> {
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
    ) -> Result<Device, DeviceError> {
        // SAFETY: Caller guaranteed the extension structs are compliant.
        unsafe { self.build_impl(Some(extension)) }
    }

    unsafe fn build_impl<E: ExtendsDeviceCreateInfo>(self, extension: Option<&mut E>) -> Result<Device, DeviceError> {
        let instance_handle = self.device.instance().handle();
        // SAFETY: The Arc<InstanceHandle> stored in the device guarantees the device will not outlive the
        // instance.
        let raw_instance = unsafe { instance_handle.raw() };

        // Select an appropriate queue.
        //
        // For the time being, we do not support the user selecting queues on their own. This is probably something we
        // want to change for the future.
        let queue_families = unsafe { raw_instance.get_physical_device_queue_family_properties(self.device.handle()) };

        // Per the Vulkan specification, if the capabilities include graphics, the queue MUST also support
        // transfer operations.
        // https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/vkspec.html#VkQueueFlags
        let (queue_family_index, _) = queue_families
            .iter()
            .enumerate()
            .find(|(_, queue)| queue.queue_flags.contains(QueueFlags::GRAPHICS))
            .ok_or(DeviceError::NoSuitableQueue)?;

        let queue_info = [DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index as u32)
            .queue_priorities(&[1.0])
            .build()];

        // Must create two vecs or else the pointers passed into vulkan will be null.
        let extensions_c = self
            .enable_extensions
            .iter()
            .map(|e| CString::new(&e[..]).expect("NUL terminated extension name"))
            .collect::<Vec<_>>();
        let extensions_ptr = extensions_c.iter().map(|c| c.as_ptr()).collect::<Vec<_>>();

        let mut create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&extensions_ptr[..]);

        if let Some(extension) = extension {
            create_info = create_info.push_next(extension);
        }

        if let Some(features) = &self.features {
            create_info = create_info.enabled_features(features);
        }

        // SAFETY: The Arc<InstanceHandle> stored in the device guarantees the device will not outlive the
        // instance.
        let device =
            unsafe { raw_instance.create_device(self.device.handle(), &create_info, None) }.map_err(VkError::from)?;

        // Now create the queue
        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        let inner = Arc::new(DeviceHandle {
            device,
            physical: unsafe { self.device.handle() },
            queue_family_index,
            queue,
            version: self.device.version(),
            enabled_extensions: self.enable_extensions.clone(),
            instance: instance_handle,
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
            features: None,
        }
    }

    /// Returns maximum the version of the API the device supports.
    ///
    /// This will be the lower of the physical device's maximum supported version and the specified version of
    /// the instance.
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

    /// Returns a reference to the underlying [`ash::Device`].
    ///
    /// # Safety
    /// - Callers must NOT destroy the returned device.
    /// - Child objects created using the device must not outlive the device
    /// (`VUID-vkDestroyDevice-device-00378`).
    ///
    /// These safety requirements may be checked by enabling validation layers.
    pub unsafe fn raw(&self) -> &ash::Device {
        unsafe { self.0.raw() }
    }

    pub fn queue_family_index(&self) -> usize {
        self.0.queue_family_index()
    }
}
