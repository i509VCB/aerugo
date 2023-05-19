use smithay::reexports::wayland_server;
use wayland_server::{
    backend::{ClientId, ObjectId},
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};

pub mod __interfaces {
    use crate::wayland::ext::foreign_toplevel::__interfaces::*;
    use smithay::reexports::wayland_server::backend as wayland_backend;
    wayland_scanner::generate_interfaces!("../protocols/aerugo-wm-v1.xml");
}
use self::{__interfaces::*, aerugo_wm_toplevel_v1::AerugoWmToplevelV1, aerugo_wm_v1::AerugoWmV1};

use crate::{shell::ToplevelId, wayland::ext::foreign_toplevel::*, Aerugo};
wayland_scanner::generate_server_code!("../protocols/aerugo-wm-v1.xml");

impl GlobalDispatch<AerugoWmV1, ()> for Aerugo {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<AerugoWmV1>,
        _: &(),
        init: &mut DataInit<'_, Self>,
    ) {
        // TODO: Store this
        let _aerugo_wm = init.init(resource, ());
    }
}

impl Dispatch<AerugoWmV1, ()> for Aerugo {
    fn request(
        state: &mut Self,
        client: &Client,
        resource: &AerugoWmV1,
        request: aerugo_wm_v1::Request,
        _: &(),
        display: &DisplayHandle,
        init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_v1::Request;

        match request {
            Request::Destroy => {}
            Request::Pong { serial: _ } => {
                // TODO: Handle ping pong
            }
            Request::GetWmToplevel { handle, id } => {
                let toplevel_id = *handle.data::<ToplevelId>().unwrap();

                // TODO: Ensure only one instance exists.
                let _wm_toplevel = init.init(id, toplevel_id);
            }
        }
    }

    fn destroyed(_state: &mut Self, _client: ClientId, _resource: ObjectId, _data: &()) {
        // TODO: Handle WM client death
    }
}

impl Dispatch<AerugoWmToplevelV1, ToplevelId> for Aerugo {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &AerugoWmToplevelV1,
        request: aerugo_wm_toplevel_v1::Request,
        _id: &ToplevelId,
        _display: &DisplayHandle,
        _init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_toplevel_v1::Request;

        match request {
            Request::Destroy => {}
        }
    }

    fn destroyed(_state: &mut Self, _client: ClientId, _resource: ObjectId, _data: &ToplevelId) {}
}
