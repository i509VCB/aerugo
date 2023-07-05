use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

use once_cell::unsync::OnceCell;
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};

use crate::{State, Toplevel, ToplevelInfo};

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

static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

// This import is essential until https://github.com/Smithay/wayland-rs/issues/623 is fixed.
use ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE;

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _conn: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        use ext_foreign_toplevel_list_v1::Event;

        match event {
            Event::Toplevel { toplevel } => {
                let id = *toplevel.data::<NonZeroU64>().unwrap();

                // Initialize extension objects
                let wm_toplevel = state.aerugo_wm_v1.get_wm_toplevel(&toplevel, queue, id);

                state.toplevels.insert(
                    id,
                    Toplevel {
                        identifier: OnceCell::new(),
                        current: ToplevelInfo {
                            app_id: None,
                            title: None,
                            capabilities: Vec::new(),
                            min_size: None,
                            max_size: None,
                            parent: None,
                            geometry: None,
                        },
                        pending: None,
                        handle: toplevel,
                        wm_toplevel,
                    },
                );
            }
            Event::Finished => {}
        }
    }

    event_created_child!(Self, ExtForeignToplevelListV1, [
        EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1,
            NonZeroU64::new(HANDLE_COUNTER.fetch_add(1, Ordering::AcqRel)).expect("effectively impossible")
        )
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, NonZeroU64> for State {
    fn event(
        state: &mut Self,
        _proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        id: &NonZeroU64,
        _conn: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        use ext_foreign_toplevel_handle_v1::Event;

        let toplevel = state.toplevels.get_mut(id).unwrap();

        match event {
            Event::Closed => {
                // TODO: Forward closed instead of just removing
                toplevel.wm_toplevel.destroy();
                toplevel.handle.destroy();
                state.toplevels.remove(id);
            }
            Event::Done => {
                if let Some(info) = toplevel.pending.take() {
                    toplevel.current = info;
                }

                dbg!(toplevel);
                // TODO: Notify that toplevel state has changed.
            }
            Event::Title { title } => {
                toplevel.pending().title = Some(title);
            }
            Event::AppId { app_id } => {
                toplevel.pending().app_id = Some(app_id);
            }
            Event::Identifier { identifier } => {
                if let Err((current, new)) = toplevel.identifier.try_insert(identifier) {
                    tracing::error!(
                        "Possible bad server implementation? handle \"{}\" had identifier changed to \"{}\"",
                        current,
                        new
                    );
                }
            }
        }
    }
}
