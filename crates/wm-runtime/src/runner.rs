use std::{fmt, io, thread};

use calloop::channel::Channel;
use wasmtime::{
    component::{Resource, ResourceAny},
    Store,
};

use crate::{
    host::{
        aerugo::wm::types::{DecorationMode, Features, ToplevelUpdates},
        exports::aerugo::wm::wm_types::WmTypes,
    },
    ConfigureUpdate, Id, ToplevelUpdate, WmEvent, WmState, WmToplevel,
};

pub struct WmRunner {
    channel: Channel<WmEvent>,
    store: Store<WmState>,
    wm: ResourceAny,
    funcs: WmTypes,
}

impl fmt::Debug for WmRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WmThread")
            .field("channel", &self.channel)
            .field("store", &self.store)
            .field("wm", &self.wm)
            .finish_non_exhaustive()
    }
}

impl WmRunner {
    pub(super) fn new(channel: Channel<WmEvent>, store: Store<WmState>, wm: ResourceAny, funcs: WmTypes) -> Self {
        Self {
            channel,
            store,
            wm,
            funcs,
        }
    }

    pub fn run(mut self) -> io::Result<()> {
        thread::Builder::new().name("aerugo wm runtime".into()).spawn(move || {
            loop {
                // Since this is run on a separate thread, we want to manually poll and suspend the thread if no
                // wm events are pending.
                match self.channel.recv() {
                    Ok(event) => {
                        // Dispatch the event on the runtime.
                        // Add some fuel for while dispatching.
                        let result = match event {
                            WmEvent::NewToplevel { toplevel, features } => self.new_toplevel(toplevel, features),
                            WmEvent::ClosedToplevel(id) => self.closed_toplevel(id),
                            WmEvent::UpdateToplevel { toplevel, update } => self.update_toplevel(toplevel, update),
                            WmEvent::AckToplevel { toplevel, serial } => todo!(),
                            WmEvent::NewOutput { output } => todo!(),
                            WmEvent::UpdateOutput { output } => todo!(),
                            WmEvent::DisconnectOutput(_) => todo!(),
                        };

                        result.expect("handle error");
                    }

                    // The other end was closed.
                    Err(_) => return,
                }
            }
        })?;

        Ok(())
    }

    // TODO: Somehow communicate all the initial state
    fn new_toplevel(&mut self, id: Id, features: Features) -> wasmtime::Result<()> {
        self.store.data_mut().toplevels.insert(
            id.rep(),
            WmToplevel {
                id,
                initial_commit: false,
                features,
                app_id: Default::default(),
                title: Default::default(),
                min_size: Default::default(),
                max_size: Default::default(),
                geometry: Default::default(),
                parent: Default::default(),
                state: Default::default(),
                decorations: DecorationMode::ClientSide,
                resize_edge: Default::default(),
            },
        );

        Ok(())
    }

    fn closed_toplevel(&mut self, id: Id) -> wasmtime::Result<()> {
        self.funcs
            .wm()
            .call_closed_toplevel(&mut self.store, self.wm, id.rep().get())
    }

    fn update_toplevel(&mut self, id: Id, update: ToplevelUpdate) -> wasmtime::Result<()> {
        let mut updates = ToplevelUpdates::default();
        let wm = self.store.data_mut();

        // Check if the parent being set is valid before borrowing the toplevel data.

        let toplevel = wm.get_toplevel(id)?;

        if (toplevel.app_id != update.app_id) && update.app_id.is_some() {
            updates |= ToplevelUpdates::APP_ID;
        }

        if (toplevel.title != update.title) && update.title.is_some() {
            updates |= ToplevelUpdates::TITLE;
        }

        if let ConfigureUpdate::Update(min_size) = update.min_size {
            updates |= ToplevelUpdates::MIN_SIZE;
            toplevel.min_size = min_size;
        }

        if let ConfigureUpdate::Update(max_size) = update.max_size {
            updates |= ToplevelUpdates::MAX_SIZE;
            toplevel.max_size = max_size;
        }

        if let ConfigureUpdate::Update(geometry) = update.geometry {
            updates |= ToplevelUpdates::GEOMETRY;
            toplevel.geometry = geometry;
        }

        if let ConfigureUpdate::Update(parent) = update.parent {
            todo!()
        }

        if let Some(state) = update.state {
            // TODO
        }

        if let Some(decorations) = update.decorations {}

        if let ConfigureUpdate::Update(edge) = update.resize_edge {
            updates |= ToplevelUpdates::REQUEST_RESIZE;
        }

        if toplevel.initial_commit {
            toplevel.initial_commit = false;
            let toplevel = Resource::new_own(toplevel.id.rep().get());

            self.funcs.wm().call_new_toplevel(&mut self.store, self.wm, toplevel)
        } else {
            self.funcs
                .wm()
                .call_update_toplevel(&mut self.store, self.wm, id.rep().get(), updates)
        }
    }
}
