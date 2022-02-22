use ash::vk;
use smithay::backend::allocator;

macro_rules! fourcc_to_vk {
    () => {
        
    };
}

const fn fourcc_to_vk(format: allocator::Fourcc) -> Option<vk::Format> {
    // The Vulkan spec states the following regarding format conversions: https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_EXT_image_drm_format_modifier.html#_format_translation
    // > DRM formats do not distinguish between RGB and sRGB (as of 2018-03-28)
    //
    // Fourcc format identifiers in big endian.
    //
    // And from the vulkan spec:
    // > The representation of non-packed formats is that the first component specified in the name of the
    // > format is in the lowest memory addresses and the last component specified is in the highest memory
    // > addresses.
    //
    // This means the endianess of Vulkan and fourcc formats are reversed.

    match format {
        // These formats have a direct Vulkan representation.
        allocator::Fourcc::Abgr8888 => Some(vk::Format::R8G8B8A8_SRGB),
        allocator::Fourcc::Argb8888 => Some(vk::Format::B8G8R8A8_SRGB),
        allocator::Fourcc::Xbgr8888 => Some(vk::Format::R8G8B8A8_SRGB), // TODO: X?
        allocator::Fourcc::Xrgb8888 => Some(vk::Format::B8G8R8A8_SRGB), // TODO: X?

        // Common formats that *may* have direct Vulkan representations.
        allocator::Fourcc::Abgr2101010 => None,
        allocator::Fourcc::Argb2101010 => None,
        allocator::Fourcc::Xbgr2101010 => None,
        allocator::Fourcc::Xrgb2101010 => None,

        // Uncommon formats that *may* have direct Vulkan representations.
        allocator::Fourcc::R8 => None,
        allocator::Fourcc::R16 => None,
        allocator::Fourcc::Rg88 => None,
        allocator::Fourcc::Rg1616 => None,
        allocator::Fourcc::Gr88 => None,
        allocator::Fourcc::Gr1616 => None,

        // Floating point formats that *may* have direct Vulkan representations (_SFLOAT).
        allocator::Fourcc::Abgr16161616f => None,
        allocator::Fourcc::Argb16161616f => None,
        allocator::Fourcc::Xbgr16161616f => None,
        allocator::Fourcc::Xrgb16161616f => None,

        // TODO: BGRA/BGRX and RGBA/BGRX formats

        // TODO: These need more research
        allocator::Fourcc::Abgr1555 => None,
        allocator::Fourcc::Abgr4444 => None,
        allocator::Fourcc::Argb1555 => None,
        allocator::Fourcc::Argb4444 => None,
        allocator::Fourcc::Axbxgxrx106106106106 => None,
        allocator::Fourcc::Ayuv => None,
        allocator::Fourcc::Bgr233 => None,
        allocator::Fourcc::Bgr565 => None,
        allocator::Fourcc::Bgr565_a8 => None,
        allocator::Fourcc::Bgr888 => None,
        allocator::Fourcc::Bgr888_a8 => None,
        allocator::Fourcc::Bgra1010102 => None,
        allocator::Fourcc::Bgra4444 => None,
        allocator::Fourcc::Bgra5551 => None,
        allocator::Fourcc::Bgra8888 => None,
        allocator::Fourcc::Bgrx1010102 => None,
        allocator::Fourcc::Bgrx4444 => None,
        allocator::Fourcc::Bgrx5551 => None,
        allocator::Fourcc::Bgrx8888 => None,
        allocator::Fourcc::Bgrx8888_a8 => None,
        allocator::Fourcc::Big_endian => None,
        allocator::Fourcc::C8 => None,
        allocator::Fourcc::Nv12 => None,
        allocator::Fourcc::Nv15 => None,
        allocator::Fourcc::Nv16 => None,
        allocator::Fourcc::Nv21 => None,
        allocator::Fourcc::Nv24 => None,
        allocator::Fourcc::Nv42 => None,
        allocator::Fourcc::Nv61 => None,
        allocator::Fourcc::P010 => None,
        allocator::Fourcc::P012 => None,
        allocator::Fourcc::P016 => None,
        allocator::Fourcc::P210 => None,
        allocator::Fourcc::Q401 => None,
        allocator::Fourcc::Q410 => None,
        allocator::Fourcc::Rgb332 => None,
        allocator::Fourcc::Rgb565 => None,
        allocator::Fourcc::Rgb565_a8 => None,
        allocator::Fourcc::Rgb888 => None,
        allocator::Fourcc::Rgb888_a8 => None,
        allocator::Fourcc::Rgba1010102 => None,
        allocator::Fourcc::Rgba4444 => None,
        allocator::Fourcc::Rgba5551 => None,
        allocator::Fourcc::Rgba8888 => None,
        allocator::Fourcc::Rgbx1010102 => None,
        allocator::Fourcc::Rgbx4444 => None,
        allocator::Fourcc::Rgbx5551 => None,
        allocator::Fourcc::Rgbx8888 => None,
        allocator::Fourcc::Rgbx8888_a8 => None,
        allocator::Fourcc::Uyvy => None,
        allocator::Fourcc::Vuy101010 => None,
        allocator::Fourcc::Vuy888 => None,
        allocator::Fourcc::Vyuy => None,
        allocator::Fourcc::X0l0 => None,
        allocator::Fourcc::X0l2 => None,
        allocator::Fourcc::Xbgr1555 => None,
        allocator::Fourcc::Xbgr4444 => None,
        allocator::Fourcc::Xbgr8888_a8 => None,
        allocator::Fourcc::Xrgb1555 => None,
        allocator::Fourcc::Xrgb4444 => None,
        allocator::Fourcc::Xrgb8888_a8 => None,
        allocator::Fourcc::Xvyu12_16161616 => None,
        allocator::Fourcc::Xvyu16161616 => None,
        allocator::Fourcc::Xvyu2101010 => None,
        allocator::Fourcc::Xyuv8888 => None,
        allocator::Fourcc::Y0l0 => None,
        allocator::Fourcc::Y0l2 => None,
        allocator::Fourcc::Y210 => None,
        allocator::Fourcc::Y212 => None,
        allocator::Fourcc::Y216 => None,
        allocator::Fourcc::Y410 => None,
        allocator::Fourcc::Y412 => None,
        allocator::Fourcc::Y416 => None,
        allocator::Fourcc::Yuv410 => None,
        allocator::Fourcc::Yuv411 => None,
        allocator::Fourcc::Yuv420 => None,
        allocator::Fourcc::Yuv420_10bit => None,
        allocator::Fourcc::Yuv420_8bit => None,
        allocator::Fourcc::Yuv422 => None,
        allocator::Fourcc::Yuv444 => None,
        allocator::Fourcc::Yuyv => None,
        allocator::Fourcc::Yvu410 => None,
        allocator::Fourcc::Yvu411 => None,
        allocator::Fourcc::Yvu420 => None,
        allocator::Fourcc::Yvu422 => None,
        allocator::Fourcc::Yvu444 => None,
        allocator::Fourcc::Yvyu => None,
    }
}
