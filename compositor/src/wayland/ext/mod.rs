//! `ext` vendored wayland protocol implementations

mod foreign_toplevel;

pub mod protocols {
    pub use super::foreign_toplevel::{ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1};
}
