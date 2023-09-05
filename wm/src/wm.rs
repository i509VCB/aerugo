use std::{
    cell::OnceCell,
    collections::{BTreeMap, VecDeque},
    io,
    num::NonZeroU32,
    ops::RangeInclusive,
    sync::atomic::{AtomicU32, Ordering},
};

use euclid::{Rect, UnknownUnit};
use rustix::io::Errno;
use wayland_backend::{client::WaylandError, protocol::ProtocolError};
use wayland_client::{
    globals::{BindError, GlobalList, GlobalListContents},
    protocol::wl_registry::{self, WlRegistry},
    Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle,
};

use crate::{
    aerugo_wm::protocol::{aerugo_wm_toplevel_v1::AerugoWmToplevelV1, aerugo_wm_v1::AerugoWmV1},
    foreign_toplevel::protocol::{
        ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
    },
    id, AlreadyDestroyed, Event, MissingGlobal, Setup, ToplevelEvent, ToplevelId,
};

static GENERATION: AtomicU32 = AtomicU32::new(1);

pub struct Protocols {
    toplevel_list: ExtForeignToplevelListV1,
    aerugo_wm: AerugoWmV1,
}

pub struct Inner {
    protocols: Protocols,

    /// Generation of this wm instance. This is retrieved from the global generation counter.
    generation: NonZeroU32,

    /// The next surface id.
    next_surface_id: NonZeroU32,

    /// The next toplevel id.
    next_toplevel_id: NonZeroU32,

    /// All toplevel instances known by this wm.
    toplevels: BTreeMap<NonZeroU32, ToplevelInfo>,

    // TODO:
    // - surfaces
    // - transactions
    queue: QueueHandle<Self>,
    pending_events: VecDeque<Event>,
}

// TODO: Remove unknown unit
#[derive(Debug)]
pub enum ToplevelUpdate {
    Title(String),
    AppId(String),
    Identifier(String),
    Geometry(Rect<i32, UnknownUnit>),
}

const AERUGO_WM_VERSION: RangeInclusive<u32> = 1..=1;
const FOREIGN_TOPLEVEL_LIST_VERSION: RangeInclusive<u32> = 1..=1;

impl Inner {
    // TODO: new
    pub fn new(conn: &Connection) -> Result<(Self, EventQueue<Self>), Setup> {
        let generation = GENERATION.fetch_add(1, Ordering::AcqRel);

        // If 0 is loaded, there have been billions of dead or failed instances. Clearly this is something we
        // can't really deal with.
        let generation = NonZeroU32::new(generation).expect("Internal generation counter overflowed");

        let next_surface_id = NonZeroU32::new(1).unwrap();
        let next_toplevel_id = NonZeroU32::new(1).unwrap();
        let toplevels = BTreeMap::new();

        let (list, queue) = wayland_client::globals::registry_queue_init(conn).expect("TODO");
        let mut missing = Vec::new();

        let toplevel_list =
            list.bind::<ExtForeignToplevelListV1, Self, ()>(&queue.handle(), FOREIGN_TOPLEVEL_LIST_VERSION, ());
        test_global(&list, &mut missing, &toplevel_list, FOREIGN_TOPLEVEL_LIST_VERSION);
        let aerugo_wm = list.bind::<AerugoWmV1, Self, ()>(&queue.handle(), AERUGO_WM_VERSION, ());
        test_global(&list, &mut missing, &aerugo_wm, AERUGO_WM_VERSION);

        if !missing.is_empty() {
            return Err(Setup::MissingGlobals(missing));
        }

        let protocols = Protocols {
            toplevel_list: toplevel_list.unwrap(),
            aerugo_wm: aerugo_wm.unwrap(),
        };

        let inner = Self {
            protocols,
            generation,
            next_surface_id,
            next_toplevel_id,
            toplevels,
            queue: queue.handle(),
            pending_events: VecDeque::new(),
        };

        Ok((inner, queue))
    }

    pub fn init_toplevel(&mut self, handle: ExtForeignToplevelHandleV1) -> id::Toplevel {
        let next = self.next_surface_id.checked_add(1).expect("overflow");

        let id = id::Toplevel {
            generation: self.generation,
            id: self.next_surface_id,
        };

        self.next_surface_id = next;

        let wm = self.protocols.aerugo_wm.get_wm_toplevel(&handle, &self.queue, id);
        self.toplevels.insert(
            id.id,
            ToplevelInfo {
                id,
                handle,
                wm,
                new_sent: false,
                identifier: OnceCell::new(),
            },
        );

        id
    }

    pub fn apply_toplevel_updates(&mut self, id: id::Toplevel) {
        let Some(toplevel) = self.toplevels.get_mut(&id.id) else {
            // TODO: Warn
            return;
        };

        if toplevel.identifier.get().is_none() {
            // TODO: Warn about no identifier and ignore the toplevel until set.
            return;
        }

        // If the initial commit has been sent, prepare the new toplevel.
        if !toplevel.new_sent {
            toplevel.new_sent = false;
            self.pending_events
                .push_back(Event::Toplevel(ToplevelEvent::New(ToplevelId(id))));
        }

        // Apply pending state with events
    }

    pub fn update_toplevel(&mut self, id: id::Toplevel, update: ToplevelUpdate) {
        let Some(toplevel) = self.toplevels.get_mut(&id.id) else {
            // TODO: Warn
            return;
        };

        dbg!(&update);

        match update {
            // TODO: Update
            ToplevelUpdate::Title(_title) => {}
            ToplevelUpdate::AppId(_app_id) => {}
            ToplevelUpdate::Identifier(identifier) => {
                if toplevel.identifier.set(identifier).is_err() {
                    // TODO: Warn about bad server impl
                }
            }
            ToplevelUpdate::Geometry(_) => {}
        }
    }

    pub fn closed_toplevel(&mut self, id: id::Toplevel) {
        self.pending_events
            .push_back(Event::Toplevel(ToplevelEvent::Closed(ToplevelId(id))));
    }

    pub fn release_toplevel(&mut self, id: id::Toplevel) -> Result<(), AlreadyDestroyed> {
        let Some(toplevel) = self.toplevels.remove(&id.id) else {
            return Err(AlreadyDestroyed);
        };

        // TODO: Clean up toplevel state related to scene graph and transactions?
        toplevel.wm.destroy();
        toplevel.handle.destroy();

        Ok(())
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        self.pending_events.pop_front()
    }
}

fn test_global<I: Proxy>(
    globals: &GlobalList,
    missing: &mut Vec<MissingGlobal>,
    result: &Result<I, BindError>,
    version: RangeInclusive<u32>,
) {
    let Err(ref err) = result else {
        return;
    };

    let interface = I::interface().name.into();

    let error = match err {
        BindError::UnsupportedVersion => {
            // Find the highest version global
            let available = globals
                .contents()
                .with_list(|globals| {
                    globals
                        .iter()
                        .filter(|global| global.interface == interface)
                        .max_by(|a, b| a.version.cmp(&b.version))
                        .map(|global| global.version)
                })
                .expect("If the version is unsupported, the global must be available at some version");
            MissingGlobal::IncompatibleVersion {
                interface,
                version: available,
                compatible: version,
            }
        }

        BindError::NotPresent => MissingGlobal::Missing(interface),
    };

    missing.push(error);
}

pub fn map_dispatch(err: DispatchError) -> io::Error {
    match err {
        wayland_client::DispatchError::BadMessage {
            sender_id,
            interface,
            opcode,
        } => {
            let protocol_id = sender_id.protocol_id();
            let message = format!("bad message from {interface}@{protocol_id} with opcode {opcode}");
            io::Error::new(io::ErrorKind::InvalidData, message)
        }
        wayland_client::DispatchError::Backend(WaylandError::Io(io)) => io,
        wayland_client::DispatchError::Backend(WaylandError::Protocol(ProtocolError {
            code,
            object_id,
            object_interface,
            message: error_message,
        })) => {
            let mut message = format!("protocol error on {object_interface}@{object_id} (error code {code})");

            // libwayland-sys does not expose the error message
            if !message.is_empty() {
                use std::fmt::Write;
                let _ = write!(message, ": {error_message}");
            }

            io::Error::from_raw_os_error(Errno::PROTO.raw_os_error())
        }
    }
}

#[derive(Debug)]
pub struct ToplevelInfo {
    id: id::Toplevel,
    handle: ExtForeignToplevelHandleV1,
    wm: AerugoWmToplevelV1,
    new_sent: bool,
    identifier: OnceCell<String>,
}

impl Dispatch<WlRegistry, GlobalListContents> for Inner {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}
