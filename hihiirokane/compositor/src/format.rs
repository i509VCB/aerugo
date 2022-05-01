use smithay::{backend::allocator::Fourcc, reexports::wayland_server::protocol::wl_shm};

macro_rules! format_tables {
    (
        $(
            $fourcc_wl: ident {
                $(opaque: $opaque: ident,)?
                $(
                    gl: $gl: ident,
                )?
                $(
                    // PACK Vulkan formats are endian dependent.
                    $(#[$vk_meta: meta])*
                    vk: $vk: ident,
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
            get_opaque_fourcc(fourcc).is_some()
        }

        /// Returns true if the wl_shm code has a alpha channel.
        pub const fn wl_has_alpha(
            wl: smithay::reexports::wayland_server::protocol::wl_shm::Format,
        ) -> bool {
            get_opaque_wl(wl).is_some()
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
                    smithay::backend::allocator::Fourcc::$fourcc_wl => Some((
                        ash::vk::Format::$vk,
                        fourcc_has_alpha(smithay::backend::allocator::Fourcc::$fourcc_wl)
                    )),
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
                    smithay::reexports::wayland_server::protocol::wl_shm::Format::$fourcc_wl => Some((
                        ash::vk::Format::$vk,
                        wl_has_alpha(smithay::reexports::wayland_server::protocol::wl_shm::Format::$fourcc_wl)
                    )),
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

        // Now conversions to other formats may be specified.
        //
        // You may specify how to convert a fourcc code to an OpenGL or Vulkan format.
        //
        // These fields are optional, omitting them indicates there is no compatible format mapping.

        // For Vulkan, we can only use SRGB formats or else we need to convert the format.
        vk: B8G8R8A8_SRGB,
    },

    Xrgb8888 {
        vk: B8G8R8A8_SRGB,
    },

    // Non-mandatory formats

    Abgr8888 {
        opaque: Xbgr8888,
        vk: R8G8B8A8_SRGB,
    },

    Xbgr8888 {
        vk: R8G8B8A8_SRGB,
    },

    // The PACK32 formats in Vulkan are equivalent to a u32 instead of a [u8; 4].
    //
    // This means these formats will depend on the host endianness.
    //
    // TODO: Validate the PACK32 Vulkan formats.
    Rgba8888 {
        opaque: Rgbx8888,
        #[cfg(target_endian = "little")]
        vk: A8B8G8R8_SRGB_PACK32,
    },

    Rgbx8888 {
        #[cfg(target_endian = "little")]
        vk: A8B8G8R8_SRGB_PACK32,
    },

    Bgr888 {
        vk: R8G8B8_SRGB,
    },

    Rgb888 {
        vk: B8G8R8_SRGB,
    },

    R8 {
        vk: R8_SRGB,
    },

    Gr88 {
        vk: R8G8_SRGB,
    },
}

pub fn fourcc_to_wl(fourcc: Fourcc) -> Option<wl_shm::Format> {
    match fourcc {
        // Manual mapping for the two mandatory formats wl_shm defines.
        //
        // Every other format should be the same as the fourcc code.
        Fourcc::Argb8888 => Some(wl_shm::Format::Argb8888),
        Fourcc::Xrgb8888 => Some(wl_shm::Format::Xrgb8888),

        fourcc => wl_shm::Format::try_from(fourcc as u32).ok(),
    }
}
