//! Wayland protocol handling
//!
//! This module contains the common Wayland protocol handling that the compositor will always support.
//!
//! Some protocols are not included in this module. Notably `wl_shm` and `zwp_linux_dmabuf_v1` since these two
//! protocols require deeper integration with the backend.

pub mod core;
pub mod ext;

pub mod aerugo_wm;
pub mod xdg_shell;
