use smithay::reexports::wayland_server;
use wayland_server::{
    backend::{ClientId, ObjectId},
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};

pub mod __interfaces {
    use crate::wayland::ext::foreign_toplevel::__interfaces::*;
    use smithay::reexports::wayland_server::backend as wayland_backend;
    use smithay::reexports::wayland_server::protocol::__interfaces::*;
    wayland_scanner::generate_interfaces!("../protocols/aerugo-wm-v1.xml");
}
use self::{
    __interfaces::*, aerugo_wm_configure_v1::AerugoWmConfigureV1, aerugo_wm_node_v1::AerugoWmNodeV1,
    aerugo_wm_surface_v1::AerugoWmSurfaceV1, aerugo_wm_toplevel_v1::AerugoWmToplevelV1,
    aerugo_wm_transaction_v1::AerugoWmTransactionV1, aerugo_wm_v1::AerugoWmV1,
};
use wayland_server::protocol::*;

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
        _client: &Client,
        resource: &AerugoWmV1,
        request: aerugo_wm_v1::Request,
        _: &(),
        _display: &DisplayHandle,
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

                let toplevel = state.shell.toplevels.get_mut(&toplevel_id).unwrap();
                let handles = toplevel.get_handles(handle.id()).unwrap();

                if handles.aerugo_toplevel.is_some() {
                    resource.post_error(
                        aerugo_wm_v1::Error::AlreadyExtended,
                        "Already extended foreign toplevel handle with aerugo_wm_toplevel_v1",
                    );
                }

                let wm_toplevel = init.init(id, toplevel_id);

                // TODO: Query more of this info.
                let capabilities = (vec![] as Vec<aerugo_wm_toplevel_v1::Capabilities>)
                    .into_iter()
                    .map(Into::<u32>::into)
                    .map(u32::to_ne_bytes)
                    .flatten()
                    .collect::<Vec<_>>();

                wm_toplevel.capabilities(capabilities);
                handles.aerugo_toplevel = Some(wm_toplevel);
                // TODO: Suggested state (such as min_size, request_fullscreen)

                // Send a done event to indicate all current info has been sent.
                handles.handle.done();
            }
            Request::GetWmSurface { surface: _, id: _ } => todo!(),
            Request::GetToplevelNode { toplevel: _, id: _ } => todo!(),
            Request::GetSurfaceNode { surface: _, id: _ } => todo!(),
            Request::CreateConfigure { id: _ } => todo!(),
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
            Request::RequestClose => todo!(),
        }
    }

    fn destroyed(_state: &mut Self, _client: ClientId, _resource: ObjectId, _data: &ToplevelId) {}
}

impl Dispatch<AerugoWmSurfaceV1, ToplevelId> for Aerugo {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &AerugoWmSurfaceV1,
        request: aerugo_wm_surface_v1::Request,
        _data: &ToplevelId,
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_surface_v1::Request;

        match request {
            Request::Destroy => todo!(),
        }
    }
}

// TODO: User data for node
impl Dispatch<AerugoWmNodeV1, ()> for Aerugo {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &AerugoWmNodeV1,
        request: aerugo_wm_node_v1::Request,
        _data: &(),
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_node_v1::Request;

        match request {
            Request::Destroy => todo!(),
        }
    }
}

// TODO: User data for transaction?
impl Dispatch<AerugoWmTransactionV1, ()> for Aerugo {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &AerugoWmTransactionV1,
        request: aerugo_wm_transaction_v1::Request,
        _data: &(),
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_transaction_v1::Request;

        match request {
            Request::Destroy => todo!(),
            Request::Dependency { dependency: _ } => todo!(),
            Request::Configure {
                toplevel: _,
                configure: _,
            } => todo!(),
            Request::Move {
                node: _,
                offset_x: _,
                offset_y: _,
            } => todo!(),
            Request::SetOutputNode { output: _, node: _ } => todo!(),
            Request::Submit => todo!(),
            Request::Cancel => todo!(),
        }
    }
}

// TODO: User data for configure?
impl Dispatch<AerugoWmConfigureV1, ()> for Aerugo {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &AerugoWmConfigureV1,
        request: aerugo_wm_configure_v1::Request,
        _data: &(),
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        use aerugo_wm_configure_v1::Request;

        match request {
            Request::Destroy => todo!(),
            Request::States { states: _ } => todo!(),
            Request::Size { width: _, height: _ } => todo!(),
            Request::Bounds { width: _, height: _ } => todo!(),
        }
    }
}
