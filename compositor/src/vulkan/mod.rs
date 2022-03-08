#![allow(dead_code)]
#![forbid(unsafe_op_in_unsafe_fn)]
/*
Conventions for contributors:

The following guidelines should be followed for code added to this backend:
1. Where appropriate, the ID of a Valid Usage statement should be placed as a comment near a Vulkan command or
   function. A Valid ID usage looks like this for example: `VUID-vkDestroyDevice-device-00378`.

   Sometimes implicit Valid Usage may be mentioned if the requirements are not obvious.
*/
// Because this is an experiment for a future pull request.
//#![warn(missing_docs)] // not as much yellow

// TODO: Specify Vulkan api version used to create instances and devices.

//! Common helper types and utilities for using the Vulkan API.
//!
//! This module contains thin wrappers over [`ash`](https://crates.io/crates/ash) to allow more easily using Vulkan
//! while not being restrictive in how Vulkan may be used. The thin wrapper addresses the following:
//! - Enumerating over all available instance extensions and layers.
//! - Creating an instance through a [`builder`](instance::Instance::builder) using requested instance
//!   extensions, layers and app info.
//! - Enumerating over all available physical devices, device capabilities, extensions/
//! - Creating a logical devices.

/* And more as we add things */

//!
//! ## How to use Vulkan
//!
//! To use this module, start by creating an instance using [`Instance::builder`](instance::Instance::builder).
//! Vulkan **is** explicit and requires you request every layer and extension you wish to use, so add the
//! names of the extensions and layers to the builder so they may be used. To complete construction of the
//! instance, call [`InstanceBuilder::build`](instance::InstanceBuilder::build).
//!
//! In a development environment, it may be useful to enable validation layers to assist with programming. You may
//! enable validation layers through either your environment variables (setting the value of `VK_INSTANCE_LAYERS`) or
//! pass the name of the validation layer[^validation] into the list of layers to be enabled.
//!
//! After creating an instance, the next step is to enumerate the physical devices available to the instance using
//! [`PhysicalDevice::enumerate`](self::physical_device::PhysicalDevice::enumerate). On most systems there may only be one suitable
//! device that is available. On systems with multiple graphics cards, the properties of each device and the supported
//! extensions may be queried to select the preferred device.
//!
//! [^validation]: [`VALIDATION_LAYER_NAME`]

pub mod device;
pub mod error;
pub mod instance;
pub mod physical_device;
pub mod version;

pub mod renderer;

use ash::Entry;
use once_cell::sync::Lazy;

use self::version::Version;

/// The name of the validation layer.
///
/// This may be passed into [`InstanceBuilder::layer`](instance::InstanceBuilder::layer) to enable validation
/// layers.
///
/// This extension should not be used in production for the following reasons:
/// 1) Validation layers are not present on most systems.
/// 2) Validation layers introduce overhead for production use.
#[cfg_attr(
    not(debug_assertions),
    deprecated = "Validation layers should not be enabled in release"
)]
pub const VALIDATION_LAYER_NAME: &str = "VK_LAYER_KHRONOS_validation";

#[derive(Debug, thiserror::Error)]
#[error("Smithay requires at least Vulkan 1.1")]
pub struct UnsupportedVulkanVersion;

const SMITHAY_VERSION: Version = Version {
    variant: 0,
    major: 0,
    minor: 3,
    patch: 0,
};

static LIBRARY: Lazy<Entry> = Lazy::new(|| unsafe { Entry::load() }.expect("failed to load vulkan library"));

// TODO: Need to set up lavapipe on CI for testing some of the basic things.
#[cfg(test)]
mod test {
    use std::error::Error;

    use crate::vulkan::{device::Device, renderer::VulkanRenderer, version::Version};

    use super::{instance::Instance, physical_device::PhysicalDevice, VALIDATION_LAYER_NAME};

    #[test]
    fn instance_with_layer() -> Result<(), Box<dyn Error>> {
        let instance = unsafe {
            Instance::builder()
                .layer(VALIDATION_LAYER_NAME)
                .api_version(Version::VERSION_1_1)
                .build()
        }
        .expect("Failed to create instance");

        let physical = PhysicalDevice::enumerate(&instance)?
            .filter(|d| {
                d.supported_extensions()
                    .iter()
                    .any(|ext| ext == "VK_EXT_physical_device_drm")
            })
            .next()
            .expect("No device supports physical device drm");

        println!("{} supporting version: {}", physical.name(), physical.version());

        if let Some(driver) = physical.driver() {
            println!("Driver info:");
            println!("\tname: {}", driver.name);
            println!("\tinfo: {}", driver.info);
            println!("\tid: {:?}", driver.id);
            println!("\tconformance: {:?}", driver.conformance)
        } else {
            println!("No driver info");
        }

        let mut device_builder = Device::builder(&physical);
        let req_extensions = VulkanRenderer::optimal_device_extensions(Version::VERSION_1_1).unwrap();

        for extension in req_extensions {
            device_builder = device_builder.extension(*extension);
        }

        let device = unsafe { device_builder.build() }?;

        let renderer = VulkanRenderer::new(&device).expect("TODO: Error type");

        //println!("DMA {:#?}", renderer.dmabuf_formats().collect::<Vec<_>>());
        //println!("SHM {:#?}", renderer.shm_formats());

        drop(device);
        drop(instance);

        Ok(())
    }
}
