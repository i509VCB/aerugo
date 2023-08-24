use std::sync::OnceLock;

use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};

use crate::{
    id,
    wm::{self, ToplevelUpdate},
};

use self::protocol::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

pub mod protocol {
    use wayland_client;

    pub mod __interfaces {
        use wayland_client::backend as wayland_backend;
        wayland_scanner::generate_interfaces!("../protocols/ext-foreign-toplevel-list-v1.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_client_code!("../protocols/ext-foreign-toplevel-list-v1.xml");
}

// This import is essential until https://github.com/Smithay/wayland-rs/issues/623 is fixed.
use ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE;

impl Dispatch<ExtForeignToplevelListV1, ()> for wm::Inner {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use ext_foreign_toplevel_list_v1::Event;

        match event {
            Event::Toplevel { toplevel } => {
                let id = state.init_toplevel(toplevel.clone());
                toplevel.data::<OnceLock<id::Toplevel>>().unwrap().set(id).unwrap();
            }
            Event::Finished => {}
        }
    }

    event_created_child!(Self, ExtForeignToplevelListV1, [
        EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, OnceLock::new())
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, OnceLock<id::Toplevel>> for wm::Inner {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        id: &OnceLock<id::Toplevel>,
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use ext_foreign_toplevel_handle_v1::Event;

        let id = *id.get().expect("id not initialized");

        match event {
            Event::Closed => state.closed_toplevel(id),
            Event::Done => state.apply_toplevel_updates(id),
            Event::Title { title } => state.update_toplevel(id, ToplevelUpdate::Title(title)),
            Event::AppId { app_id } => state.update_toplevel(id, ToplevelUpdate::AppId(app_id)),
            Event::Identifier { identifier } => state.update_toplevel(id, ToplevelUpdate::Identifier(identifier)),
        }
    }
}
