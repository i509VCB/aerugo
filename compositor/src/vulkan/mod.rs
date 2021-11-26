#![allow(dead_code)] // Because this is an experiment for a future pull request.
#![warn(missing_docs)]
// TODO: Specify Vulkan api version used to create instances and devices.

//! Common helper types and utilities for using the Vulkan API.
//!
//! This module contains thin wrappers over [`ash`](https://crates.io/crates/ash) to allow more easily using Vulkan
//! while not being restrictive in how Vulkan may be used. The thin wrapper addresses the following:
//! - Enumerating over all available instance extensions and layers.
//! - Creating an instance through a [`builder`](InstanceBuilder) using requested instance extensions, layers and app
//! info.
//! - Enumerating over all available physical devices, device capabilities, extensions and creating logical devices.
//!
//! ## How to use Vulkan
//!
//! To use this module, start by creating an instance using [`Instance::builder`]. Vulkan **is** explicit and requires
//! you request every layer and extension you wish to use, so add the names of the extensions and layers to the builder
//! so they may be used. To complete construction of the instance, call [`InstanceBuilder::build`].
//!
//! In a development environment, it may be useful to enable validation layers to assist with programming. You may
//! enable validation layers through either your environment variables (setting the value of `VK_INSTANCE_LAYERS`) or
//! pass the name of the validation layer[^validation] into the list of layers to be enabled.
//!
//! After creating an instance, the next step is to enumerate the physical devices available to the instance using
//! [`PhysicalDevice::enumerate`]. On most systems there may only be one suitable
//! device that is available. On systems with multiple graphics cards, the properties of each device and the supported
//! extensions may be queried to select the preferred device.
//!
//! [^validation]: [`VALIDATION_LAYER_NAME`]

mod device;
mod physical_device;
mod queue;

use std::{
    cmp::Ordering,
    error::Error,
    ffi::{c_void, CStr, CString, NulError},
    fmt::{self, Display, Formatter},
    mem,
    sync::Arc,
};

use ash::{
    vk::{ApplicationInfo, DebugUtilsMessengerCreateInfoEXT, InstanceCreateInfo},
    Entry,
};
use lazy_static::lazy_static;
use slog::Logger;

pub use self::device::Device;
pub use self::physical_device::PhysicalDevice;
pub use self::queue::QueueFamily;

const SMITHAY_VERSION: Version = Version {
    variant: 0,
    major: 0,
    minor: 3,
    patch: 0,
};

/// The name of the validation layer.
///
/// This may be passed into [`InstanceBuilder::layer`] to enable validation layers.
///
/// This extension should not be used in production for the following reasons:
/// 1) Validation layers are not present on most systems
/// 2) Validation layers introduce overhead for production use
pub const VALIDATION_LAYER_NAME: &str = "VK_LAYER_KHRONOS_validation";

/// A Vulkan API version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    /// The variant of the Vulkan API.
    ///
    /// Generally this value will be `0` because the Vulkan specification uses variant `0`.
    pub variant: u32,
    /// The major version of the Vulkan API.
    pub major: u32,
    /// The minor version of the Vulkan API.
    pub minor: u32,
    /// The patch version of the Vulkan API.
    ///
    /// Most Vulkan API calls which take a version typically ignore the patch value. Consumers of the Vulkan API may
    /// typically ignore the patch value.
    pub patch: u32,
}

impl Version {
    /// Version 1.0 of the Vulkan API.
    pub const VERSION_1_0: Version = Version::from_raw(ash::vk::API_VERSION_1_0);

    /// Version 1.1 of the Vulkan API.
    pub const VERSION_1_1: Version = Version::from_raw(ash::vk::API_VERSION_1_1);

    /// Version 1.2 of the Vulkan API.
    pub const VERSION_1_2: Version = Version::from_raw(ash::vk::API_VERSION_1_2);

    /// Converts a packed version into a version struct.
    pub const fn from_raw(raw: u32) -> Version {
        Version {
            variant: ash::vk::api_version_variant(raw),
            major: ash::vk::api_version_major(raw),
            minor: ash::vk::api_version_minor(raw),
            patch: ash::vk::api_version_patch(raw),
        }
    }

    /// Converts a version struct into a packed version.
    pub const fn to_raw(self) -> u32 {
        ash::vk::make_api_version(self.variant, self.major, self.minor, self.patch)
    }

    /// Returns an object which implements [`Display`] that may be used to display a version.
    ///
    /// The `display_variant` parameter states whether the [`Version::variant`] should be displayed.
    pub fn display(&self, display_variant: bool) -> impl Display + '_ {
        VersionDisplayer {
            version: self,
            variant: display_variant,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.variant.partial_cmp(&other.variant) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }

        match self.major.partial_cmp(&other.major) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }

        match self.minor.partial_cmp(&other.minor) {
            Some(Ordering::Equal) => {}
            ord => return ord,
        }

        self.patch.partial_cmp(&other.patch)
    }
}

/// Returns the max Vulkan API version supported any created instances.
pub fn max_instance_version() -> Result<Version, InstanceError> {
    Ok(LIBRARY
        .try_enumerate_instance_version()?
        .map(Version::from_raw)
        // Vulkan 1.0 does not have `vkEnumerateInstanceVersion`.
        .unwrap_or(Version {
            variant: 0,
            major: 1,
            minor: 0,
            patch: 0,
        }))
}

/// Enumerates over the available instance layers on the system.
pub fn enumerate_layers() -> Result<impl Iterator<Item = String>, InstanceError> {
    Ok(LIBRARY
        .enumerate_instance_layer_properties()?
        .into_iter()
        .map(|properties| {
            // SAFETY: String is null terminated.
            let c_str = unsafe { CStr::from_ptr(&properties.layer_name as *const _) };
            c_str.to_str().expect("Invalid UTF-8 in layer name").to_owned()
        }))
}

/// Enumerates over the available instance layers on the system.
pub fn enumerate_extensions() -> Result<impl Iterator<Item = String>, InstanceError> {
    Ok(LIBRARY
        .enumerate_instance_extension_properties()?
        .into_iter()
        .map(|properties| {
            // SAFETY: String is null terminated.
            let c_str = unsafe { CStr::from_ptr(&properties.extension_name as *const _) };
            c_str.to_str().expect("Invalid UTF-8 in extension name").to_owned()
        }))
}

/// An error that may occur when using or creating an instance.
#[derive(Debug)]
pub enum InstanceError {
    /// The driver does not support the requested Vulkan API version.
    IncompatibleDriver,

    /// The host or device has run out of memory.
    OutOfMemory,

    /// Some requested extensions of layers are not available.
    MissingExtensionsOrLayers(MissingExtensionsOrLayers),

    /// Some other error occurred.
    Other(ash::vk::Result),
}

impl From<ash::vk::Result> for InstanceError {
    fn from(err: ash::vk::Result) -> Self {
        match err {
            ash::vk::Result::ERROR_INCOMPATIBLE_DRIVER => InstanceError::IncompatibleDriver,
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => InstanceError::OutOfMemory,
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => InstanceError::OutOfMemory,
            err => InstanceError::Other(err),
        }
    }
}

impl Display for InstanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceError::IncompatibleDriver => write!(f, "incompatible driver"),
            InstanceError::OutOfMemory => write!(f, "out of memory"),
            InstanceError::MissingExtensionsOrLayers(missing) => missing.fmt(f),
            InstanceError::Other(err) => err.fmt(f),
        }
    }
}

impl Error for InstanceError {}

/// Some requested extensions and or layers were not available when creating an instance.
#[derive(Debug)]
pub struct MissingExtensionsOrLayers {
    missing_extensions: Vec<String>,
    missing_layers: Vec<String>,
}

impl MissingExtensionsOrLayers {
    /// Returns the requested extensions that were not present when constructing an instance.
    pub fn missing_extensions(&self) -> Option<Vec<String>> {
        if self.missing_extensions.is_empty() {
            None
        } else {
            Some(self.missing_extensions.clone())
        }
    }

    /// Returns the requested layers that were not present when constructing an instance.
    pub fn missing_layers(&self) -> Option<Vec<String>> {
        if self.missing_layers.is_empty() {
            None
        } else {
            Some(self.missing_layers.clone())
        }
    }
}

impl Display for MissingExtensionsOrLayers {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.missing_extensions.is_empty() {
            writeln!(
                f,
                "instance extensions not present: ({}) ",
                self.missing_extensions.join(", ")
            )?;
        }

        if !self.missing_layers.is_empty() {
            writeln!(f, "instance layers not present: ({}) ", self.missing_layers.join(", "))?;
        }

        Ok(())
    }
}

impl Error for MissingExtensionsOrLayers {}

impl From<MissingExtensionsOrLayers> for InstanceError {
    fn from(err: MissingExtensionsOrLayers) -> Self {
        InstanceError::MissingExtensionsOrLayers(err)
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
    /// The default value is [`Version::VERSION_1_0`].
    ///
    /// You should ensure the version you are requesting is supported using [`max_instance_version`].
    pub fn api_version(mut self, version: Version) -> InstanceBuilder {
        self.api_version = version;
        self
    }

    /// Adds an instance extension to be requested when creating an [`Instance`].
    ///
    /// The extension must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the extension you are requesting is supported is to check if your extension is listed in
    /// [`enumerate_extensions`].
    pub fn extension(mut self, extension: impl Into<String>) -> InstanceBuilder {
        self.enable_extensions.push(extension.into());
        self
    }

    /// Adds an instance layer to be requested when creating an [`Instance`].
    ///
    /// The layer must be supported by the Vulkan runtime or else building the instance will fail. A great way to
    /// ensure the layer you are requesting is supported is to check if your layer is listed in [`enumerate_layers`].
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
    pub fn build(self, logger: Logger) -> Result<Instance, InstanceError> {
        // Check if the requested extensions and layers are supported.
        let supported_layers = enumerate_layers()?.collect::<Vec<_>>();
        let supported_extensions = enumerate_extensions()?.collect::<Vec<_>>();

        let enable_debug_messenger = supported_layers
            .iter()
            .any(|layer_name| layer_name == VALIDATION_LAYER_NAME);

        let missing_extensions = self
            .enable_extensions
            .iter()
            // Filter out entries that are present.
            .filter(|s| !supported_extensions.contains(s))
            .cloned()
            .collect::<Vec<_>>();

        let missing_layers = self
            .enable_layers
            .iter()
            // Filter out entries that are present.
            .filter(|s| !supported_layers.contains(s))
            .cloned()
            .collect::<Vec<_>>();

        if !missing_extensions.is_empty() || !missing_layers.is_empty() {
            return Err(MissingExtensionsOrLayers {
                missing_extensions,
                missing_layers,
            }
            .into());
        }

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

        let extensions_ptr = extensions.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let layers_ptr = layers.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();

        let mut app_info = ApplicationInfo::builder()
            .api_version(self.api_version.to_raw())
            .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"Smithay\0") })
            .engine_version(SMITHAY_VERSION.to_raw());

        if let Some(app_version) = self.app_version {
            app_info = app_info.application_version(app_version.to_raw());
        }

        let app_name = self
            .app_name
            .map(|name| CString::new(name).expect("app name contains null terminator"))
            // Yes this is ugly and probably wrong...
            .unwrap_or_else(|| CString::new("").unwrap());

        app_info = app_info.application_name(&app_name);

        let mut create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions_ptr[..])
            .enabled_layer_names(&layers_ptr[..]);

        let messenger_logger = Box::into_raw(Box::new(logger));

        let mut debug_messenger = DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
            )
            // Box up the log and pass it as user data to obtain in the callback.
            .user_data(messenger_logger as *mut _)
            .pfn_user_callback(Some(vulkan_debug_utils_callback));

        if enable_debug_messenger {
            create_info = create_info.push_next(&mut debug_messenger);
        }

        let instance = unsafe { LIBRARY.create_instance(&create_info, None) }?;
        let instance = Arc::new(InstanceInner {
            instance,
            version: self.api_version,
            messenger_logger: messenger_logger as *mut _,
        });

        Ok(instance.into())
    }
}

/// A Vulkan instance which allows interfacing with the Vulkan APIs.
#[derive(Debug)]
pub struct Instance(pub(crate) Arc<InstanceInner>);

impl Instance {
    /// Returns a builder that may be used to create an instance
    pub fn builder() -> InstanceBuilder {
        InstanceBuilder {
            api_version: Version::VERSION_1_0,
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

    /// Returns a raw handle to the underlying [`ash::Instance`].
    ///
    /// The returned handle may be used to access portions of the Vulkan API not in scope of the abstractions in this
    /// module.
    ///
    /// # Safety
    /// - The instance must not be destroyed.
    /// - The caller must guarantee usage of the handle and any objects created using the instance do not exceed the
    /// lifetime this instance.
    pub unsafe fn handle(&self) -> ash::Instance {
        self.0.instance.clone()
    }
}

pub(crate) struct InstanceInner {
    instance: ash::Instance,
    version: Version,
    messenger_logger: *mut c_void,
}

impl fmt::Debug for InstanceInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("InstanceInner").field(&self.instance.handle()).finish()
    }
}

impl From<Arc<InstanceInner>> for Instance {
    fn from(inner: Arc<InstanceInner>) -> Self {
        Instance(inner)
    }
}

impl Drop for InstanceInner {
    fn drop(&mut self) {
        // SAFETY: Wrapping the inner instance in `Arc` ensures external synchronization per Vulkan specification.
        unsafe { self.instance.destroy_instance(None) };
        // SAFETY: Drop the logger we turn into a raw pointer that is passed as user data to the debug messenger.
        unsafe { Box::<Logger>::from_raw(self.messenger_logger as *mut _) };
    }
}

#[derive(Debug)]
struct VersionDisplayer<'a> {
    version: &'a Version,
    variant: bool,
}

impl Display for VersionDisplayer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.version.major, self.version.minor, self.version.patch
        )?;

        if self.variant {
            write!(f, " variant {}", self.version.variant)?;
        }

        Ok(())
    }
}

lazy_static! {
    static ref LIBRARY: Entry = Entry::new();
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    ty: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut std::ffi::c_void,
) -> ash::vk::Bool32 {
    // The user data contains our logger always.
    let logger = Box::<Logger>::from_raw(user_data as *mut _);

    let message = CStr::from_ptr((*p_callback_data).p_message).to_string_lossy();

    slog::info!(logger, "{}", message;
        "type" => format!("{:?}", ty), "severity" => format!("{:?}", severity)
    );

    // Immediately drop the logger.
    mem::forget(logger);

    // Per the Vulkan specification, applications must ALWAYS return false.
    ash::vk::FALSE
}

// TODO: Need to set up lavapipe on CI for testing some of the basic things.
#[cfg(test)]
mod test {
    use std::error::Error;

    use slog::Logger;

    use crate::vulkan::Device;

    use super::{physical_device::PhysicalDevice, Instance, VALIDATION_LAYER_NAME};

    #[test]
    fn instance() {
        let _instance = Instance::builder()
            .build(Logger::root(slog::Discard, slog::o!()))
            .expect("Failed to create instance");
    }

    #[test]
    fn instance_with_layer() -> Result<(), Box<dyn Error>> {
        let instance = Instance::builder()
            .layer(VALIDATION_LAYER_NAME)
            .build(Logger::root(slog::Discard, slog::o!()))
            .expect("Failed to create instance");

        let physical = PhysicalDevice::enumerate(&instance)?
            .filter(|d| {
                d.supported_extensions()
                    .iter()
                    .any(|ext| ext == "VK_EXT_physical_device_drm")
            })
            .next()
            .expect("No device supports physical device drm");

        println!(
            "{} supporting version: {}",
            physical.name(),
            physical.version().display(true)
        );

        for (index, family) in physical.queue_families().enumerate() {
            println!("Queue Family {} {{", index);
            println!("\tcount: {}", family.queue_count());
            println!("\tflags: {:?}", family.flags());
            println!("}}");
        }

        let _device = Device::builder(&physical)
            .create_queue(&physical.queue_families().next().unwrap())
            .build()?;

        Ok(())
    }
}
