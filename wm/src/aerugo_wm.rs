use std::num::NonZeroU64;

use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::State;

use self::protocol::{
    aerugo_wm_node_v1::{self, AerugoWmNodeV1},
    aerugo_wm_surface_v1::{self, AerugoWmSurfaceV1},
    aerugo_wm_toplevel_v1::{self, AerugoWmToplevelV1},
    aerugo_wm_transaction_v1::{self, AerugoWmTransactionV1},
    aerugo_wm_v1::{self, AerugoWmV1},
};

pub mod protocol {
    use wayland_client;

    pub mod __interfaces {
        use crate::foreign_toplevel::protocol::__interfaces::*;
        use wayland_client::backend as wayland_backend;
        use wayland_client::protocol::__interfaces::*;
        wayland_scanner::generate_interfaces!("../protocols/aerugo-wm-v1.xml");
    }
    use self::__interfaces::*;
    use crate::foreign_toplevel::protocol::*;
    use wayland_client::protocol::*;

    wayland_scanner::generate_client_code!("../protocols/aerugo-wm-v1.xml");
}

impl Dispatch<AerugoWmV1, ()> for State {
    fn event(
        _state: &mut Self,
        wm: &AerugoWmV1,
        event: aerugo_wm_v1::Event,
        _: &(),
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use aerugo_wm_v1::Event;

        match event {
            Event::Ping { serial } => {
                // Respond to the ping so that the server does not kill the wm client.
                wm.pong(serial);
            }
        }
    }
}

impl Dispatch<AerugoWmToplevelV1, NonZeroU64> for State {
    fn event(
        _state: &mut Self,
        _proxy: &AerugoWmToplevelV1,
        event: aerugo_wm_toplevel_v1::Event,
        _id: &NonZeroU64,
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use aerugo_wm_toplevel_v1::Event;

        match event {
            Event::Capabilities { capabilities: _ } => todo!(),
            Event::MinSize { width: _, height: _ } => todo!(),
            Event::MaxSize { width: _, height: _ } => todo!(),
            Event::RequestSetMinimized => todo!(),
            Event::RequestSetMaximized => todo!(),
            Event::RequestUnsetMaximized => todo!(),
            Event::RequestSetFullscreen => todo!(),
            Event::RequestUnsetFullscreen => todo!(),
            Event::ShowWindowMenu { seat: _, x: _, y: _ } => todo!(),
            Event::SetParent { parent: _ } => todo!(),
            Event::Move { seat: _ } => todo!(),
            Event::Resize { seat: _ } => todo!(),
            Event::Geometry {
                x: _,
                y: _,
                width: _,
                length: _,
            } => todo!(),
        }
    }
}

// TODO: User data for surface?
impl Dispatch<AerugoWmSurfaceV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &AerugoWmSurfaceV1,
        event: aerugo_wm_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {}
    }
}

// TODO: User data for node?
impl Dispatch<AerugoWmNodeV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &AerugoWmNodeV1,
        event: aerugo_wm_node_v1::Event,
        _data: &(),
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {}
    }
}

// TODO: User data for transaction?
impl Dispatch<AerugoWmTransactionV1, ()> for State {
    fn event(
        _state: &mut Self,
        _proxy: &AerugoWmTransactionV1,
        event: aerugo_wm_transaction_v1::Event,
        _data: &(),
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use aerugo_wm_transaction_v1::Event;

        match event {
            Event::Applied => todo!(),
            Event::Failed => todo!(),
        }
    }
}
