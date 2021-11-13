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
//! After creating an instance, the next step is to enumerate the physical devices available to the instance. On most
//! systems there may only be one suitable device that is available. On systems with multiple graphics cards, the
//! properties of each device and the supported extensions may be queried to select the preferred device.
//!
//! [^validation]: [`VALIDATION_LAYER_NAME`]

pub mod physical_device;

use std::{
    cmp::Ordering,
    error::Error,
    ffi::{CStr, CString, NulError},
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use ash::{
    vk::{ApplicationInfo, InstanceCreateInfo},
    Entry,
};
use lazy_static::lazy_static;

/// The name of the validation layer.
///
/// This may be passed into [`InstanceBuilder::layer`] to enable validation layers.
///
/// This extension should not be used in production for the following reasons:
/// 1) Validation layers are not present on every system
/// 2) Validation layers introduce some overhead.
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
    /// ensure the layer you are requesting is supported is to check if your extension is listed in
    /// [`enumerate_layers`].
    pub fn layer(mut self, layer: impl Into<String>) -> InstanceBuilder {
        self.enable_layers.push(layer.into());
        self
    }

    /// Creates an instance using this builder.
    pub fn build(self) -> Result<Instance, InstanceError> {
        // Check if the requested extensions and layers are supported.
        {
            let supported_layers = enumerate_layers()?.collect::<Vec<_>>();
            let supported_extensions = enumerate_extensions()?.collect::<Vec<_>>();

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

        let app_info = ApplicationInfo::builder()
            .api_version(self.api_version.to_raw())
        //    .application_name(application_name) // TODO
        //    .application_version(application_version) // TODO
            .engine_name(unsafe { CStr::from_bytes_with_nul_unchecked(b"Smithay\0") }) // TODO
        //    .engine_version(engine_version) // TODO
        ;

        let create_info = InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions_ptr[..])
            .enabled_layer_names(&layers_ptr[..]);

        let instance = unsafe { LIBRARY.create_instance(&create_info, None) }?;
        let instance = Arc::new(InstanceInner { instance });

        Ok(instance.into())
    }
}

#[derive(Debug)]
pub struct Instance {
    inner: Arc<InstanceInner>,
}

impl Instance {
    /// Returns a builder that may be used to create an instance
    pub fn builder() -> InstanceBuilder {
        InstanceBuilder {
            api_version: Version::VERSION_1_0,
            enable_extensions: vec![],
            enable_layers: vec![],
        }
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
        self.inner.instance.clone()
    }
}

pub(crate) struct InstanceInner {
    instance: ash::Instance,
}

impl fmt::Debug for InstanceInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("InstanceInner").field(&self.instance.handle()).finish()
    }
}

impl From<Arc<InstanceInner>> for Instance {
    fn from(inner: Arc<InstanceInner>) -> Self {
        Instance { inner }
    }
}

impl Drop for InstanceInner {
    fn drop(&mut self) {
        // SAFETY: Wrapping the inner instance in `Arc` ensures external synchronization per Vulkan specification.
        unsafe { self.instance.destroy_instance(None) };
    }
}

lazy_static! {
    static ref LIBRARY: Entry = Entry::new();
}

// TODO: Need to set up lavapipe on CI for testing some of the basic things.
// #[cfg(test)]
// mod test {
//     use super::{Instance, VALIDATION_LAYER_NAME};

//     #[test]
//     fn instance() {
//         let _instance = Instance::builder().build().expect("Failed to create instance");
//     }

//     #[test]
//     fn instance_with_layer() {
//         let _instance = Instance::builder()
//             .layer(VALIDATION_LAYER_NAME)
//             .build()
//             .expect("Failed to create instance");
//     }
// }
