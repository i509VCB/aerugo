macro_rules! format_tables {
    (
        $(
            $fourcc_wl: ident {
                $(opaque: $opaque: ident,)?
                alpha: $alpha: expr,
                $(
                    // The meta fragment specifier exists because the in memory representation of packed
                    // formats depend on the host endianness.
                    gl:
                    $(#[$gl_meta: meta])* $gl: ident,
                )?
                $(
                    // The meta fragment specifier exists because the in memory representation of `PACK32`
                    // formats depend on the host endianness since pixels are interpreted as a u32.
                    vk:
                    $(#[$vk_meta: meta])* $vk: ident,
                )?
            }
        ),* $(,)?
    ) => {
        pub fn formats() -> impl ExactSizeIterator<Item = smithay::backend::allocator::Fourcc> {
            [
                $(
                    smithay::backend::allocator::Fourcc::$fourcc_wl,
                )*
            ]
            .into_iter()
        }

        /// Returns an equivalent fourcc code that is opaque.
        ///
        /// An opaque code will generally have padding instead of an alpha value.
        pub const fn get_opaque_fourcc(
            fourcc: smithay::backend::allocator::Fourcc,
        ) -> Option<smithay::backend::allocator::Fourcc> {
            match fourcc {
                $($(
                    smithay::backend::allocator::Fourcc::$fourcc_wl
                        => Some(smithay::backend::allocator::Fourcc::$opaque),
                )*)*

                _ => None,
            }
        }

        /// Returns an equivalent wl_shm code that is opaque.
        ///
        /// An opaque code will generally have padding instead of an alpha value.
        pub const fn get_opaque_wl(
            fourcc: smithay::reexports::wayland_server::protocol::wl_shm::Format,
        ) -> Option<smithay::reexports::wayland_server::protocol::wl_shm::Format> {
            match fourcc {
                $($(
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::$fourcc_wl
                        => Some(smithay::reexports::wayland_server::protocol::wl_shm::Format::$opaque),
                )*)*

                _ => None,
            }
        }

        /// Returns true if the fourcc code has a alpha channel.
        pub const fn fourcc_has_alpha(
            fourcc: smithay::backend::allocator::Fourcc,
        ) -> bool {
            match fourcc {
                $(
                    smithay::backend::allocator::Fourcc::$fourcc_wl => $alpha,
                )*

                _ => false,
            }
        }

        /// Returns true if the wl_shm code has a alpha channel.
        pub const fn wl_has_alpha(
            fourcc: smithay::reexports::wayland_server::protocol::wl_shm::Format,
        ) -> bool {
            match fourcc {
                $(
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::$fourcc_wl => $alpha,
                )*

                _ => false,
            }
        }

        /// Returns an equivalent Vulkan format from the specified fourcc code.
        ///
        /// The second field of the returned tuple describes whether Vulkan needs to swizzle the alpha
        /// component.
        pub const fn fourcc_to_vk(
            fourcc: smithay::backend::allocator::Fourcc,
        ) -> Option<(ash::vk::Format, bool)> {
            match fourcc {
                $($(
                    $(#[$vk_meta])*
                    smithay::backend::allocator::Fourcc::$fourcc_wl => Some((ash::vk::Format::$vk, $alpha)),
                )*)*

                _ => None
            }
        }

        /// Returns an equivalent Vulkan format from the specified wl_shm code.
        ///
        /// The second field of the returned tuple describes whether Vulkan needs to swizzle the alpha
        /// component.
        pub const fn wl_shm_to_vk(
            wl: smithay::reexports::wayland_server::protocol::wl_shm::Format,
        ) -> Option<(ash::vk::Format, bool)> {
            match wl {
                $($(
                    $(#[$vk_meta])*
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::$fourcc_wl
                        => Some((ash::vk::Format::$vk, $alpha)),
                )*)*

                _ => None
            }
        }
    };
}

format_tables! {
    // Formats mandated by wl_shm

    // Using the first entry as a reference, this is how the syntax works:
    //
    // The first thing we declare is fourcc code. The fourcc code should appear before opening the braces.
    Argb8888 {
        // Some formats may have an opaque equivalent where the alpha component is used as padding.
        opaque: Xrgb8888,

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
        alpha: false,
        vk: B8G8R8A8_SRGB,
    },

    // Non-mandatory formats

    Abgr8888 {
        opaque: Xbgr8888,
        alpha: true,
        vk: R8G8B8A8_SRGB,
    },

    Xbgr8888 {
        alpha: false,
        vk: R8G8B8A8_SRGB,
    },

    // The PACK32 formats in Vulkan are equivalent to a u32 instead of a [u8; 4].
    //
    // This means these formats will depend on the host endianness.
    //
    // TODO: Validate the PACK32 Vulkan formats.
    Rgba8888 {
        opaque: Rgbx8888,
        alpha: true,
        vk: #[cfg(target_endian = "little")] A8B8G8R8_SRGB_PACK32,
    },

    Rgbx8888 {
        alpha: false,
        vk: #[cfg(target_endian = "little")] A8B8G8R8_SRGB_PACK32,
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
}

pub fn fourcc_to_wl(
    fourcc: smithay::backend::allocator::Fourcc,
) -> Option<smithay::reexports::wayland_server::protocol::wl_shm::Format> {
    match fourcc {
        // Manual mapping for the two mandatory formats wl_shm defines.
        //
        // Every other format should be the same as the fourcc code.
        smithay::backend::allocator::Fourcc::Argb8888 => {
            Some(smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888)
        }
        smithay::backend::allocator::Fourcc::Xrgb8888 => {
            Some(smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888)
        }

        fourcc => smithay::reexports::wayland_server::protocol::wl_shm::Format::from_raw(fourcc as u32),
    }
}