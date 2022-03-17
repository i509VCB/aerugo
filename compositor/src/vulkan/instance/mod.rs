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

use super::{error::VkError, version::Version, LIBRARY, SMITHAY_VERSION};

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

        if let Some(debug) = &mut self.debug {
            unsafe {
                debug
                    .debug_utils
                    .destroy_debug_utils_messenger(debug.debug_utils_messenger, None)
            }
        }

        unsafe {
            self.handle.destroy_instance(None);
        }
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
    pub unsafe fn build(mut self) -> Result<Instance, InstanceError> {
        // We require at least Vulkan 1.1
        if self.api_version < Version::VERSION_1_1 {
            return Err(InstanceError::UnsupportedVulkanVersion(self.api_version));
        }

        // Check if the requested extensions and layers are supported.
        let supported_layers = Instance::enumerate_layers()?.collect::<Vec<_>>();
        let supported_extensions = Instance::enumerate_extensions()?.collect::<Vec<_>>();

        let mut supports_debug = false;

        if supported_extensions.iter().any(|ext| ext == "VK_EXT_debug_utils") {
            // TODO: Make this nicer to enable
            self.enable_extensions.push("VK_EXT_debug_utils".to_owned());
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

        let create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_ptrs[..])
            .enabled_extension_names(&extension_ptrs[..]);

        // SAFETY(VUID-vkCreateInstance-ppEnabledExtensionNames-01388): The caller has guaranteed the requirements.
        // SAFETY: The Entry will always outlive the instance since it is a static variable.
        let instance = unsafe { LIBRARY.create_instance(&create_info, None) }.map_err(VkError::from)?;

        let debug = if supports_debug {
            // FIXME: This probably needs to be gated?
            // Now setup the debug utils if it's available.
            let debug_utils = DebugUtils::new(&LIBRARY, &instance);
            let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
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
                .pfn_user_callback(Some(vulkan_debug_utils_callback));

            let debug_utils_messenger =
                unsafe { debug_utils.create_debug_utils_messenger(&debug_create_info, None) }.map_err(VkError::from)?;

            Some(DebugState {
                debug_utils,
                debug_utils_messenger,
            })
        } else {
            None
        };

        let handle = Arc::new(InstanceHandle {
            handle: instance,
            version: self.api_version,
            enabled_extensions: self.enable_extensions,
            debug,
        });

        Ok(Instance(handle))
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let message = unsafe { ffi::CStr::from_ptr((*p_callback_data).p_message) };
    let severity = format!("{:?}", message_severity).to_lowercase();
    let ty = format!("{:?}", message_type).to_lowercase();
    println!("[Debug][{}][{}] {:?}", severity, ty, message);

    // Must always return false.
    vk::FALSE
}

/// A Vulkan instance which allows interfacing with the Vulkan APIs.
#[derive(Debug)]
pub struct Instance(pub(crate) Arc<InstanceHandle>);

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

struct DebugState {
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
