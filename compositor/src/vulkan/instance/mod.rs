mod error;

use std::{
    ffi::{self, c_void, CStr, CString, NulError},
    fmt::{self, Formatter},
    sync::Arc,
};

use ash::{
    extensions::ext::DebugUtils,
    vk::{self, ApplicationInfo, InstanceCreateInfo},
};

use super::{
    error::VkError,
    physical_device::{PhysicalDevice, PhysicalDeviceInner},
    version::Version,
    LIBRARY, SMITHAY_VERSION,
};

pub use self::error::*;

/// Wrapper around [`ash::Instance`] to ensure an instance is only destroyed once all resources have been
/// dropped.
///
/// This object also contains the [`version`](InstanceHandle::version) of the instance.
pub struct InstanceHandle {
    handle: ash::Instance,
    version: Version,
    enabled_extensions: Vec<String>,
    debug: Option<DebugState>,
    logger: slog::Logger,
}

impl InstanceHandle {
    /// Returns a reference to the underlying [`ash::Instance`].
    ///
    /// Take care when using the underlying type, since all the valid usage requirements in the Vulkan
    /// specification apply.
    ///
    /// In particular, keep in mind that child objects created using the instance must not outlive the
    /// instance (`VUID-vkDestroyInstance-instance-00629`).
    ///
    /// The valid usage requirements may be checked by enabling validation layers.
    pub fn raw(&self) -> &ash::Instance {
        &self.handle
    }

    /// Returns the version of the instance.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns a list of enabled instance extensions for this instance.
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.enabled_extensions.clone()
    }

    /// Returns true if the specified instance extension is enabled.
    pub fn is_extension_enabled(&self, extension: &str) -> bool {
        self.enabled_extensions.iter().any(|supported| supported == extension)
    }
}

impl fmt::Debug for InstanceHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstanceHandle")
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

// A Vulkan instance may be used on any thread.
//
// The contents of the handle are also constant.
unsafe impl Send for InstanceHandle {}
unsafe impl Sync for InstanceHandle {}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        // SAFETY: The Vulkan specification states the following requirements:
        //
        // > VUID-vkDestroyInstance-instance-00629: All child objects created using instance must have been
        // destroyed prior to destroying instance. Accessing the handle is unsafe, and callers must guarantee
        // no child objects outlive the instance.
        //
        // > Host access to instance must be externally synchronized.
        // Host access is externally synchronized since the InstanceHandle is given to users inside an Arc.
        let messenger_logger = if let Some(debug) = &mut self.debug {
            unsafe {
                debug
                    .debug_utils
                    .destroy_debug_utils_messenger(debug.debug_utils_messenger, None)
            }

            // Since InstanceCreateInfo is extended with DebugUtilsMessengerCreateInfoEXT, destroying the
            // instance will mean the logger is used.
            Some(unsafe { Box::from_raw(debug.logger_ptr) })
        } else {
            None
        };

        unsafe {
            self.handle.destroy_instance(None);
        }

        // Now that the instance has been destroyed, we can destroy the logger.
        drop(messenger_logger);
    }
}

/// A builder used to construct an [`Instance`].
///
/// To instantiate, use [`Instance::builder`].
#[derive(Debug)]
pub struct InstanceBuilder {
    api_version: Version,
    enable_extensions: Vec<String>,
    enable_layers: Vec<String>,
    app_name: Option<String>,
    app_version: Option<Version>,
}

impl InstanceBuilder {
    /// Sets the API version that should be used when creating an instance.
    ///
    /// The default value is [`Version::VERSION_1_1`].
    ///
    /// You should ensure the version you are requesting is supported using [`Instance::max_instance_version`].
    ///
    /// Note that Smithay requires at least Vulkan 1.1.
    pub fn api_version(mut self, version: Version) -> InstanceBuilder {
        self.api_version = version;
        self
    }

    /// Adds an instance extension to be requested when creating an [`Instance`].
    ///
    /// The extension must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the extension you are requesting is supported is to check if your extension is listed in
    /// [`Instance::enumerate_extensions`].
    ///
    /// If available, the builder will try to enable the `VK_EXT_debug_utils`.
    pub fn extension(mut self, extension: impl Into<String>) -> InstanceBuilder {
        self.enable_extensions.push(extension.into());
        self
    }

    /// Adds an instance layer to be requested when creating an [`Instance`].
    ///
    /// The layer must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the layer you are requesting is supported is to check if your layer is listed in [`Instance::enumerate_layers`].
    pub fn layer(mut self, layer: impl Into<String>) -> InstanceBuilder {
        self.enable_layers.push(layer.into());
        self
    }

    /// Sets the app name that will be used by the driver when creating an instance.
    pub fn app_name(mut self, name: impl Into<String>) -> InstanceBuilder {
        self.app_name = Some(name.into());
        self
    }

    /// Sets the app version that will be used by the driver when creating an instance.
    pub fn app_version(mut self, version: Version) -> InstanceBuilder {
        self.app_version = Some(version);
        self
    }

    /// Creates an instance using this builder.
    ///
    /// # Safety
    ///
    /// The valid usage requirement for vkCreateInstance, `VUID-vkCreateInstance-ppEnabledExtensionNames-01388`,
    /// states all enabled extensions must also enable the required dependencies.
    pub unsafe fn build(mut self, logger: slog::Logger) -> Result<Instance, InstanceError> {
        // We require at least Vulkan 1.1
        if self.api_version < Version::VERSION_1_1 {
            return Err(InstanceError::UnsupportedVulkanVersion(self.api_version));
        }

        // Check if the requested extensions and layers are supported.
        let supported_layers = Instance::enumerate_layers()?.collect::<Vec<_>>();
        let supported_extensions = Instance::enumerate_extensions()?.collect::<Vec<_>>();

        let mut supports_debug = false;

        for extension in RECOMMENDED_INSTANCE_EXTENSIONS {
            if supported_extensions.iter().any(|ext| ext == *extension) {
                self.enable_extensions.push(extension.to_string());
            }
        }

        // Check if we can enable logging machinery
        if supported_extensions.iter().any(|ext| ext == "VK_EXT_debug_utils") {
            supports_debug = true;
        }

        let missing_layers = self
            .enable_layers
            .iter()
            // Filter out entries that are present.
            .filter(|s| !supported_layers.contains(s))
            .cloned()
            .collect::<Vec<_>>();

        if !missing_layers.is_empty() {
            return Err(InstanceError::MissingLayers(missing_layers));
        }

        let missing_extensions = self
            .enable_extensions
            .iter()
            // Filter out entries that are present.
            .filter(|s| !supported_extensions.contains(s))
            .cloned()
            .collect::<Vec<_>>();

        if !missing_extensions.is_empty() {
            return Err(InstanceError::MissingExtensions(missing_extensions));
        }

        // We cannot immediately go to a Vec<*const c_char> since that will cause the CString drop impl to
        // be called and our resulting pointers will have been freed.
        let extensions = self
            .enable_extensions
            .iter()
            .map(|s| CString::new(s.clone()))
            .collect::<Result<Vec<_>, NulError>>()
            .expect("Non UTF-8 extension string");

        let layers = self
            .enable_layers
            .iter()
            .map(|s| CString::new(s.clone()))
            .collect::<Result<Vec<_>, NulError>>()
            .expect("Non UTF-8 layer string");

        let mut app_info = ApplicationInfo::builder()
            .api_version(self.api_version.to_raw())
            // SAFETY: Vulkan requires a NUL terminated C string.
            .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"Smithay\0") })
            .engine_version(SMITHAY_VERSION.to_raw());

        if let Some(app_version) = self.app_version {
            app_info = app_info.application_version(app_version.to_raw());
        }

        let app_name = self
            .app_name
            .map(|name| CString::new(name).expect("app name contains null terminator"));

        if let Some(app_name) = &app_name {
            app_info = app_info.application_name(app_name);
        }

        let layer_ptrs = layers.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let extension_ptrs = extensions.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();

        let messenger_logger = logger.new(slog::o!("vulkan" => "debug_messenger"));

        // Allocate the logger on the heap for Vulkan.
        let messenger_logger_ptr = Box::into_raw(Box::new(messenger_logger.clone()));

        let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback))
            .user_data(messenger_logger_ptr as *mut _);

        let mut create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_ptrs[..])
            .enabled_extension_names(&extension_ptrs[..]);

        if supports_debug {
            create_info = create_info.push_next(&mut debug_create_info);
        }

        // SAFETY(VUID-vkCreateInstance-ppEnabledExtensionNames-01388): The caller has guaranteed the requirements.
        // SAFETY: The Entry will always outlive the instance since it is a static variable.
        let instance = unsafe { LIBRARY.create_instance(&create_info, None) }.map_err(|err| {
            // Destroy the logger pointer in order to not leak memory
            let _ = unsafe { Box::from_raw(messenger_logger_ptr) };

            VkError::from(err)
        })?;

        let debug = if supports_debug {
            let debug_utils = DebugUtils::new(&LIBRARY, &instance);

            let debug_utils_messenger =
                unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None) }.map_err(VkError::from)?;

            Some(DebugState {
                logger_ptr: messenger_logger_ptr,
                debug_utils,
                debug_utils_messenger,
            })
        } else {
            None
        };

        let logger = logger.new(slog::o!("vulkan" => "instance"));

        slog::info!(logger, "Created new instance" ; slog::o!("version" => format!("{}", self.api_version)));
        slog::info!(logger, "Enabled instance layers: {:?}", self.enable_layers);
        slog::info!(logger, "Enabled instance extensions: {:?}", self.enable_extensions);

        let handle = Arc::new(InstanceHandle {
            handle: instance,
            version: self.api_version,
            enabled_extensions: self.enable_extensions,
            debug,
            logger: logger.clone(),
        });

        // Physical device enumeration:

        let enumerated_devices = unsafe { handle.raw().enumerate_physical_devices() }.map_err(VkError::from)?;
        let mut physical_devices = Vec::with_capacity(enumerated_devices.len());

        for (index, phy) in enumerated_devices.iter().enumerate() {
            match PhysicalDeviceInner::new(handle.raw(), *phy) {
                Ok(phy) => {
                    slog::info!(
                        logger,
                        "Found physical device #{} ({} api: {})",
                        index,
                        &phy.device_name,
                        &phy.api_version
                    );

                    let logger = logger.new(slog::o!("device" => phy.device_name.to_string()));

                    if let Some(driver_info) = &phy.driver_info {
                        slog::info!(
                            logger,
                            "Driver info (name: {}, info: {}, id: {:?})",
                            driver_info.name,
                            driver_info.info,
                            driver_info.id
                        );
                    }

                    if let Some(primary_node) = &phy.primary_node {
                        slog::info!(
                            logger,
                            "Physical device primary node {}:{}",
                            primary_node.major(),
                            primary_node.minor(),
                        );
                    }

                    if let Some(render_node) = &phy.render_node {
                        slog::info!(
                            logger,
                            "Physical device render node {}:{}",
                            render_node.major(),
                            render_node.minor(),
                        );
                    }

                    physical_devices.push(phy);
                }

                Err(err) => {
                    slog::error!(logger, "Failed to query information about physical device #{}", index ; "err" => format!("{}", err));
                    continue;
                }
            }
        }

        Ok(Instance(handle, physical_devices))
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    logger: *mut c_void,
) -> vk::Bool32 {
    // Get the logger from the user data pointer we gave to Vulkan.
    //
    // The logger is allocated on the heap using a box, but we do not want to drop the logger, so read from
    // the pointer.
    let logger: &slog::Logger = unsafe { (logger as *mut slog::Logger).as_ref() }.unwrap();

    let message = unsafe { ffi::CStr::from_ptr((*p_callback_data).p_message) };
    let message = format!("{:?}", message).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => slog::debug!(logger, "{}", message ; "ty" => ty),
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => slog::trace!(logger, "{}", message ; "ty" => ty),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => slog::warn!(logger, "{}", message ; "ty" => ty),
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => slog::error!(logger, "{}", message ; "ty" => ty),
        _ => (),
    }

    // Must always return false.
    vk::FALSE
}

/// A Vulkan instance which allows interfacing with the Vulkan APIs.
#[derive(Debug)]
pub struct Instance(pub(crate) Arc<InstanceHandle>, Vec<PhysicalDeviceInner>);

impl Instance {
    /// Returns the max Vulkan API version supported any created instances.
    pub fn max_instance_version() -> Result<Version, VkError> {
        let version = LIBRARY
            .try_enumerate_instance_version()?
            .map(Version::from_raw)
            // Vulkan 1.0 does not have `vkEnumerateInstanceVersion`.
            .unwrap_or(Version::VERSION_1_0);

        Ok(version)
    }

    /// Enumerates over the available instance layers on the system.
    pub fn enumerate_layers() -> Result<impl Iterator<Item = String>, VkError> {
        let layers = LIBRARY
            .enumerate_instance_layer_properties()?
            .into_iter()
            .map(|properties| {
                // SAFETY: Vulkan guarantees the string is null terminated.
                let c_str = unsafe { CStr::from_ptr(&properties.layer_name as *const _) };
                c_str.to_str().expect("Invalid UTF-8 in layer name").to_owned()
            });

        Ok(layers)
    }

    /// Enumerates over the available instance layers on the system.
    pub fn enumerate_extensions() -> Result<impl Iterator<Item = String>, VkError> {
        let extensions = LIBRARY
            .enumerate_instance_extension_properties()?
            .into_iter()
            .map(|properties| {
                // SAFETY: Vulkan guarantees the string is null terminated.
                let c_str = unsafe { CStr::from_ptr(&properties.extension_name as *const _) };
                c_str.to_str().expect("Invalid UTF-8 in extension name").to_owned()
            });

        Ok(extensions)
    }

    /// Returns a builder that may be used to create an instance
    pub fn builder() -> InstanceBuilder {
        InstanceBuilder {
            api_version: Version::VERSION_1_1,
            enable_extensions: vec![],
            enable_layers: vec![],
            app_name: None,
            app_version: None,
        }
    }

    /// Returns the version of the API the instance has been created with.
    pub fn version(&self) -> Version {
        self.0.version
    }

    /// Returns a list of enabled instance extensions for this instance.
    pub fn enabled_extensions(&self) -> Vec<String> {
        self.0.enabled_extensions()
    }

    /// Returns true if the specified instance extension is enabled.
    pub fn is_extension_enabled(&self, extension: &str) -> bool {
        self.0.is_extension_enabled(extension)
    }

    pub fn enumerate_devices(&self) -> impl Iterator<Item = PhysicalDevice<'_>> {
        self.1.iter().map(|inner| PhysicalDevice { inner })
    }

    /// Returns a handle to the underling [`ash::Instance`].
    ///
    /// The Vulkan API enforces a strict lifetimes over objects that are created, meaning child objects
    /// cannot outlive their instance. A great way to ensure the instance will live long enough is storing a
    /// handle inside the container of child objects. This handle will automatically destroy the instance
    /// when the reference count reaches zero.
    pub fn handle(&self) -> Arc<InstanceHandle> {
        self.0.clone()
    }

    /// Returns a reference to the underlying [`ash::Instance`].
    ///
    /// Take care when using the underlying type, since all the valid usage requirements in the Vulkan
    /// specification apply.
    ///
    /// In particular, keep in mind that child objects created using the instance must not outlive the
    /// instance (`VUID-vkDestroyInstance-instance-00629`).
    ///
    /// The valid usage requirements may be checked by enabling validation layers.
    pub fn raw(&self) -> &ash::Instance {
        self.0.raw()
    }
}

/// Instance extensions that we load if they are available.
///
/// These extensions aren't mandatory but are nice to have.
const RECOMMENDED_INSTANCE_EXTENSIONS: &[&str] = &["VK_EXT_debug_utils"];

struct DebugState {
    /// Pointer to the logger.
    ///
    /// Allocated on the heap as a [`Box`].
    logger_ptr: *mut slog::Logger,
    debug_utils: DebugUtils,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl fmt::Debug for DebugState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebugState")
            .field("debug_utils_messenger", &self.debug_utils_messenger)
            .finish_non_exhaustive()
    }
}
