//! Wayland protocol handling
//!
//! This module contains the common Wayland protocol handling that the compositor will always support.
//!
//! Some protocols are not included in this module. Notably `wl_shm` and `zwp_linux_dmabuf_v1` since these two
//! protocols require deeper integration with the backend.

mod core;
mod ext;

mod xdg_shell;
