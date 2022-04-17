#![allow(dead_code)]
#![deny(unsafe_op_in_unsafe_fn)]
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

/*
  For maintainers and contributors:

  One of the most useful ways to understand Vulkan is the specification found here: https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/.
  You can also search up the values of constants and types to get more information.

  # A bird's eye view

  The Vulkan backend is a mid-level abstraction to handle common actions, such as creating an instance, obtaining a list
  of supported extensions, enumerating devices and other miscellaneous things that would be useful for a compositor
  to query. The Vulkan backend explicitly does not try to turn every stone. To leave an escape hatch, types should
  provide a way to access the underlying types from ash.

  # Safety considerations

  There is a LOT of unsafe code that results from ash being 99% unsafe code. The general rules of thumb with
  unsafe code still apply (see the nomicon). Vulkan as a specification also requires you uphold specific API
  invariants.

  A few of the invariants you must uphold at runtime in Vulkan include:
  - Only using features and constants provided by the Vulkan when the required extensions are enabled or present.
    (see https://www.khronos.org/registry/vulkan/specs/1.3-extensions/html/chap3.html#fundamentals-validusage-extensions).
  - Ensure all lifetime requirements objects have are valid.
  - Ensure the external synchronization requirements are met when an object requires external synchronization.
  - Ensure all "Valid Usage" requirements in the specification are met.

  In order to keep our sanity with verifying unsafe code, you should probably do the following:
  - Where appropriate, the ID of a "Valid Usage" statement should be placed as a comment near a Vulkan command or
    function. A Valid ID usage looks like this for example: `VUID-vkDestroyDevice-device-00378`.
    These IDs may be easy searched up and are a valid reason to justify that a function is in fact, safe.
  - Where unclear, implicit "Valid Usage" may be mentioned.
*/

pub mod device;
pub mod error;
pub mod instance;
pub mod physical_device;
pub mod version;

pub mod allocator;
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
    use std::{error::Error, sync::Mutex};

    use slog::Drain;
    use smithay::{
        backend::renderer::{Bind, ImportMem, ImportMemWl, Renderer},
        utils::Transform,
    };

    use crate::vulkan::{device::Device, renderer::VulkanRenderer, version::Version};

    use super::{instance::Instance, VALIDATION_LAYER_NAME};

    #[test]
    fn instance_with_layer() -> Result<(), Box<dyn Error>> {
        let logger = slog::Logger::root(Mutex::new(slog_term::term_full().fuse()).fuse(), slog::o!());

        let instance = unsafe {
            Instance::builder()
                .layer(VALIDATION_LAYER_NAME)
                .api_version(Version::VERSION_1_1)
                .build(logger.clone())
        }
        .expect("Failed to create instance");

        let physical = instance
            .enumerate_devices()
            .filter(|d| {
                d.supported_extensions()
                    .iter()
                    .any(|ext| ext == "VK_EXT_physical_device_drm")
            })
            .next()
            .expect("No device supports physical device drm");

        let mut device_builder = Device::builder(&physical);
        let req_extensions = VulkanRenderer::optimal_device_extensions();

        for extension in req_extensions {
            device_builder = device_builder.extension(*extension);
        }

        let device = unsafe { device_builder.build(&instance) }?;

        let mut renderer = VulkanRenderer::new(&device).expect("TODO: Error type");

        // println!("DMA Render {:#?}", renderer.dmabuf_render_formats().collect::<Vec<_>>());
        // println!(
        //     "DMA Texture {:#?}",
        //     renderer.dmabuf_texture_formats().collect::<Vec<_>>()
        // );
        println!("shm formats: {:?}", renderer.shm_formats());

        let texture = renderer
            .import_memory(&[0xFF, 0xFF, 0xFF, 0xFF], (1, 1).into(), false)
            .expect("import");

        renderer.bind(texture.clone()).expect("bind");

        renderer
            .render((1, 1).into(), Transform::Normal, |_renderer, _frame| {})
            .expect("render");

        // TODO: Keep ownership of the value during bind.
        drop(texture);

        Ok(())
    }
}
