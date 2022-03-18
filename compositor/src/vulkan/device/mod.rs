mod error;

use std::{
    ffi::CString,
    fmt::{self, Formatter},
    sync::Arc,
};

use ash::vk::{
    self, DeviceCreateInfo, DevicePrivateDataCreateInfoEXT, DeviceQueueCreateInfo, ExtendsDeviceCreateInfo, QueueFlags,
};

use super::{
    error::VkError,
    instance::{Instance, InstanceHandle},
    physical_device::PhysicalDevice,
    Version,
};

pub use self::error::*;

pub struct DeviceHandle {
    device: ash::Device,
    pub(crate) phy: ash::vk::PhysicalDevice,
    queue_family_index: usize,
    queue: ash::vk::Queue,
    version: Version,
    enabled_extensions: Vec<String>,
    pub(crate) instance: Arc<InstanceHandle>,
}

impl DeviceHandle {
    /// Returns a reference to the underlying [`ash::Device`].
    ///
    /// Take care when using the underlying type, since all the valid usage requirements in the Vulkan
    /// specification apply.
    ///
    /// In particular, keep in mind that child objects created using the device must not outlive the
    /// device (`VUID-vkDestroyDevice-device-00378`).
    ///
    /// The valid usage requirements may be checked by enabling validation layers.
    pub fn raw(&self) -> &ash::Device {
        &self.device
    }

    pub fn phy(&self) -> &vk::PhysicalDevice {
        &self.phy
    }

    pub fn queue(&self) -> &vk::Queue {
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
    /// Adds an instance extension to be requested when creating an [`Instance`](super::instance::Instance).
    ///
    /// The extension must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the extension you are requesting is supported is to check if your extension is listed in
    /// [`Instance::enumerate_extensions`](super::instance::Instance::enumerate_extensions).
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
    ///
    /// # Safety
    ///
    /// The valid usage requirement for vkCreateDevice, `VUID-vkCreateDevice-ppEnabledExtensionNames-01387`,
    /// states all enabled extensions must also enable the required dependencies.
    ///
    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/vkspec.html#extendingvulkan-extensions-extensiondependencies>
    pub unsafe fn build(self, instance: &Instance) -> Result<Device, DeviceError> {
        // SAFETY(VUID-VkDeviceCreateInfo-pNext-pNext): None means the pNext field is a null pointer
        //
        // DevicePrivateDataCreateInfoEXT is used for monomorphization purposes. None is passed as the
        // extension, so the generic should be ignored.
        unsafe { self.build_impl::<DevicePrivateDataCreateInfoEXT>(instance, None) }
    }

    /// Returns a new device using the parameters passed into the builder.
    ///
    /// This function also supports passing extension structs for additional information to be used when creating the
    /// device. To pass multiple extension structs, use the `push_next` function of the root extension struct.
    ///
    /// # Safety
    ///
    /// The valid usage requirement for vkCreateDevice, `VUID-vkCreateDevice-ppEnabledExtensionNames-01387`,
    /// states all enabled extensions must also enable the required dependencies.
    ///
    /// The extension struct must conform to valid usage requirements in the Vulkan specification.
    pub unsafe fn build_with_extension<T: ExtendsDeviceCreateInfo>(
        self,
        instance: &Instance,
        extension: &mut T,
    ) -> Result<Device, DeviceError> {
        // SAFETY: Caller guarantees extensions conform to valid usage requirements.
        unsafe { self.build_impl(instance, Some(extension)) }
    }

    unsafe fn build_impl<E: ExtendsDeviceCreateInfo>(
        self,
        instance: &Instance,
        extension: Option<&mut E>,
    ) -> Result<Device, DeviceError> {
        let instance_handle = instance.handle();
        let raw_instance = instance_handle.raw();

        // Select an appropriate queue.
        //
        // For the time being, we do not support the user selecting queues on their own. This is probably something we
        // want to change for the future.
        let queue_families = unsafe { raw_instance.get_physical_device_queue_family_properties(self.device.handle()) };

        // If the capabilities include graphics, the queue must also support transfer operations.
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

        // SAFETY(VUID-vkDestroyInstance-instance-00629): The Arc<InstanceHandle> stored in the device
        // guarantees the device will not outlive the instance.
        //
        // SAFETY(VUID-vkCreateDevice-ppEnabledExtensionNames-01387): The caller has guaranteed the requirements.
        let device =
            unsafe { raw_instance.create_device(self.device.handle(), &create_info, None) }.map_err(VkError::from)?;

        // Now create the queue
        let queue = unsafe { device.get_device_queue(queue_family_index as u32, 0) };

        let inner = Arc::new(DeviceHandle {
            device,
            phy: unsafe { self.device.handle() },
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
    /// handle inside the container of child objects. This handle will automatically destroy the device
    /// when the reference count reaches zero.
    pub fn handle(&self) -> Arc<DeviceHandle> {
        self.0.clone()
    }

    /// Returns a reference to the underlying [`ash::Device`].
    ///
    /// Take care when using the underlying type, since all the valid usage requirements in the Vulkan
    /// specification apply.
    ///
    /// In particular, keep in mind that child objects created using the device must not outlive the
    /// device (`VUID-vkDestroyDevice-device-00378`).
    ///
    /// The valid usage requirements may be checked by enabling validation layers.
    pub fn raw(&self) -> &ash::Device {
        self.0.raw()
    }

    pub fn queue_family_index(&self) -> usize {
        self.0.queue_family_index()
    }
}
