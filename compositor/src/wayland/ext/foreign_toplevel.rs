//! Implementation for the `ext-foreign-toplevel` family of protocols.

// ext-foreign-toplevel-list-v1 is not yet part of wayland-protocols so we need to generate it

// Re-export only the actual code, and then only use this re-export
// The `generated` module below is just some boilerplate to properly isolate stuff
// and avoid exposing internal details.
//
// You can use all the types from my_protocol as if they went from `wayland_client::protocol`.
pub use generated::{ext_foreign_toplevel_handle_v1, ext_foreign_toplevel_list_v1};
use wayland_server::{
    backend::{ClientId, ObjectId},
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};

use crate::{
    shell::{ForeignToplevelInstance, ToplevelId},
    AerugoCompositor, ClientData, PrivilegedGlobals,
};

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
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<ExtForeignToplevelListV1>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let instance = data_init.init(resource, ());
        let _instance =
            state
                .shell
                .foreign_toplevel_instances
                .entry(instance.id())
                .or_insert(ForeignToplevelInstance {
                    instance,
                    stopped: false,
                });

        // TODO: Send toplevels to instance.
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        ClientData::get_data(&client)
            .map(|data| data.is_visible(PrivilegedGlobals::FOREIGN_TOPLEVEL_LIST))
            .unwrap_or(false)
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for AerugoCompositor {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ExtForeignToplevelListV1,
        request: ext_foreign_toplevel_list_v1::Request,
        _data: &(),
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // in tree generated protocol
        #[allow(unreachable_patterns)]
        match request {
            ext_foreign_toplevel_list_v1::Request::Stop => {
                let Some(instance) = state.shell.foreign_toplevel_instances.get_mut(&resource.id()) else {
                    return;
                };

                instance.stopped = true;
            }
            ext_foreign_toplevel_list_v1::Request::Destroy => {
                // Dispatch::destroyed handles cleanup
            }

            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: ObjectId, _data: &()) {
        let _ = state.shell.foreign_toplevel_instances.remove(&resource);
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ToplevelId> for AerugoCompositor {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &ExtForeignToplevelHandleV1,
        request: ext_foreign_toplevel_handle_v1::Request,
        _data: &ToplevelId,
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // in tree generated protocol
        #[allow(unreachable_patterns)]
        match request {
            ext_foreign_toplevel_handle_v1::Request::Destroy => {
                // TODO: Check for invalid destruction order in extension protocols.
            }

            _ => unreachable!(),
        }
    }

    fn destroyed(_state: &mut Self, _client: ClientId, _resource: ObjectId, _data: &ToplevelId) {
        // TODO: Handle cleanup by removing the handle object from the instance so updates aren't sent until
        // remapped
    }
}