//! Implementation for the `ext-foreign-toplevel` family of protocols.

// TODO: Move this out of here
#![allow(non_upper_case_globals, non_camel_case_types)]

// ext-foreign-toplevel-list-v1 is not yet part of wayland-protocols so we need to generate it

// Re-export only the actual code, and then only use this re-export
// The `generated` module below is just some boilerplate to properly isolate stuff
// and avoid exposing internal details.
//
// You can use all the types from my_protocol as if they went from `wayland_client::protocol`.
use wayland_server::{backend::ClientId, Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::{
    shell::{ForeignToplevelInstance, ToplevelId},
    Aerugo, ClientData, PrivilegedGlobals,
};

use self::{
    ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1, ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
};

use smithay::reexports::wayland_server;

#[allow(non_upper_case_globals)]
pub mod __interfaces {
    use smithay::reexports::wayland_server::backend as wayland_backend;
    wayland_scanner::generate_interfaces!("../protocols/ext-foreign-toplevel-list-v1.xml");
}
use self::__interfaces::*;

wayland_scanner::generate_server_code!("../protocols/ext-foreign-toplevel-list-v1.xml");

impl GlobalDispatch<ExtForeignToplevelListV1, ()> for Aerugo {
    fn bind(
        state: &mut Self,
        display: &DisplayHandle,
        client: &Client,
        resource: New<ExtForeignToplevelListV1>,
        _global_data: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        let instance = init.init(resource, ());
        let instance = state
            .shell
            .foreign_toplevel_instances
            .entry(instance.id())
            .or_insert(ForeignToplevelInstance {
                instance,
                stopped: false,
            });

        let mut new_handles = Vec::with_capacity(state.shell.toplevels.len());

        // Create all toplevel handle instances to ensure that extension protocols do not refer to handles
        // that were not yet created.
        for toplevel in state.shell.toplevels.values_mut() {
            new_handles.push((
                toplevel.create_handle(state.generation, &instance.instance, display, client),
                toplevel,
            ));
        }

        // Now describe the toplevels.
        for (handle, toplevel) in new_handles {
            toplevel.initialize_handle(&handle);
        }
    }

    fn can_view(client: Client, _global_data: &()) -> bool {
        ClientData::get_data(&client)
            .map(|data| data.is_visible(PrivilegedGlobals::FOREIGN_TOPLEVEL_LIST))
            .unwrap_or(false)
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for Aerugo {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ExtForeignToplevelListV1,
        request: ext_foreign_toplevel_list_v1::Request,
        _: &(),
        _display: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
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

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ExtForeignToplevelListV1, _data: &()) {
        // TODO
        //let _ = state.shell.foreign_toplevel_instances.remove(&resource);
    }
}

impl Dispatch<ExtForeignToplevelHandleV1, ToplevelId> for Aerugo {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &ExtForeignToplevelHandleV1,
        request: ext_foreign_toplevel_handle_v1::Request,
        id: &ToplevelId,
        _display: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        // in tree generated protocol
        #[allow(unreachable_patterns)]
        match request {
            ext_foreign_toplevel_handle_v1::Request::Destroy => {
                // TODO: Check for invalid destruction order in extension protocols.
                if let Some(toplevel) = state.shell.toplevels.get_mut(id) {
                    if let Some(_handles) = toplevel.get_handles(resource.id()) {
                        // TODO
                    }
                }
            }

            _ => unreachable!(),
        }
    }

    fn destroyed(state: &mut Self, _client: ClientId, resource: &ExtForeignToplevelHandleV1, data: &ToplevelId) {
        if let Some(toplevel) = state.shell.toplevels.get_mut(data) {
            // TODO
            //toplevel.remove_handle(resource);
        };
    }
}
