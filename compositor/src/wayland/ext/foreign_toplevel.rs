//! Implementation for the `ext-foreign-toplevel` family of protocols.

// ext-foreign-toplevel-list-v1 is not yet part of wayland-protocols so we need to generate it

// Re-export only the actual code, and then only use this re-export
// The `generated` module below is just some boilerplate to properly isolate stuff
// and avoid exposing internal details.
//
// You can use all the types from my_protocol as if they went from `wayland_client::protocol`.
pub use generated::{ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New};

use crate::{shell::ToplevelId, AerugoCompositor};

use self::{
    ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
};

mod generated {
    use smithay::reexports::wayland_server;

    pub mod __interfaces {
        use smithay::reexports::wayland_server::backend as wayland_backend;
        wayland_scanner::generate_interfaces!("protocols/ext-foreign-toplevel-list-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("protocols/ext-foreign-toplevel-list-v1.xml");
}

impl GlobalDispatch<ExtForeignToplevelListV1, ()> for AerugoCompositor {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<ExtForeignToplevelListV1>,
        global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        todo!()
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for AerugoCompositor {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &ExtForeignToplevelListV1,
        request: ext_foreign_toplevel_list_v1::Request,
        data: &(),
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            ext_foreign_toplevel_list_v1::Request::Stop => todo!(),
            ext_foreign_toplevel_list_v1::Request::Destroy => todo!(),
        }
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ToplevelId> for AerugoCompositor {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &ExtForeignToplevelHandleV1,
        request: ext_foreign_toplevel_handle_v1::Request,
        data: &ToplevelId,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            ext_foreign_toplevel_handle_v1::Request::Destroy => todo!(),
        }
    }
}
