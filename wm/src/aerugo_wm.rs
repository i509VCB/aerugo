use std::num::NonZeroU64;

use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::State;

use self::protocol::{
    aerugo_wm_toplevel_v1::{self, AerugoWmToplevelV1},
    aerugo_wm_v1::{self, AerugoWmV1},
};

pub mod protocol {
    use wayland_client;

    pub mod __interfaces {
        use crate::foreign_toplevel::protocol::__interfaces::*;
        use wayland_client::backend as wayland_backend;
        wayland_scanner::generate_interfaces!("../protocols/aerugo-wm-v1.xml");
    }
    use self::__interfaces::*;
    use crate::foreign_toplevel::protocol::*;

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
        match event {}
    }
}
