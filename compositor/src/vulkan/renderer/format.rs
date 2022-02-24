use ash::vk;
use smithay::{
    backend::{allocator, renderer::gles2::ffi::types::GLuint},
    reexports::wayland_server::protocol::wl_shm,
};

struct FormatEntry {
    fourcc: allocator::Fourcc,
    shm: wl_shm::Format,
    gl: Option<GLuint>,
    vk: Option<vk::Format>,
}

macro_rules! format_tables {
    (
        $(
            $fourcc_wl: ident {
                alpha: $alpha: expr,
                $(gl: $gl: ident,)?
                $(vk: $vk: ident,)?
            }
        ),* $(,)?
    ) => {

    };
}

format_tables! {
    // Formats mandated by wl_shm

    // Using the first entry as a reference, this is how the syntax works:
    //
    // The first thing we declare is fourcc code. The fourcc code should appear before opening the braces.
    Argb8888 {
        // Next we need to provide data as to whether the color format has an alpha channel.
        //
        // This is a required value. Some renderers do not have specific no-alpha formats but support
        // indicating which color channels should be used.
        //
        // For example, Vulkan does not have specific formats to indicate there is a padding byte where the
        // alpha channel would exist in another format. Vulkan however allows specifying which color
        // components to use in an image view via the VkComponentSwizzle enum, allowing the alpha channel to
        // be disabled.
        alpha: true,
        // Now conversions to other formats may be specified.
        //
        // You may specify how to convert a fourcc code to an OpenGL or Vulkan format.
        //
        // These fields are optional, omitting them indicates there is no compatible format mapping.

        // For Vulkan, we can only use SRGB formats or else we need to convert the format.
        vk: B8G8R8A8_SRGB,
    },

    Xrgb8888 {
        alpha: true,
        vk: B8G8R8A8_SRGB,
    },

    // Non-mandatory formats

    Abgr8888 {
        alpha: true,
        vk: R8G8B8A8_SRGB,
    },

    Xbgr8888 {
        alpha: false,
        vk: R8G8B8A8_SRGB,
    },

    Rgba8888 {
        alpha: true,
        vk: A8B8G8R8_SRGB,
    },

    Rgbx8888 {
        alpha: true,
        vk: A8B8G8R8_SRGB,
    },

    Bgr888 {
        alpha: false,
        vk: R8G8B8_SRGB,
    },

    Rgb888 {
        alpha: false,
        vk: B8G8R8_SRGB,
    },

    R8 {
        alpha: false,
        vk: R8_SRGB,
    },

    Gr88 {
        alpha: false,
        vk: R8G8_SRGB,
    },

    // § 3.9.3. 16-Bit Floating-Point Numbers
    //
    // > 16-bit floating point numbers are defined in the “16-bit floating point numbers” section of the
    // > Khronos Data Format Specification.
    //
    // The khronos data format defines a 16-bit floating point number as a half precision IEEE 754-2008 float
    // (binary16).
    //
    // Since the DRM Fourcc formats that are floating point are also IEEE-754, Vulkan can represent some
    // floating point Drm Fourcc formats.

    Abgr16161616f {
        alpha: true,
        vk: R16G16B16A16_SFLOAT,
    },

    Xbgr16161616f {
        alpha: false,
        vk: R16G16B16A16_SFLOAT,
    }
}
