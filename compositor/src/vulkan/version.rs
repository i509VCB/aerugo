use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
};

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
