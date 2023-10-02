//! Wasm WM runtime for the Aerugo.

mod host;
mod id;
mod runner;

use std::{
    collections::HashMap,
    fmt::{self, Display},
    num::NonZeroU32,
};

use calloop::{
    channel::{Channel, Sender},
    EventSource, Poll, PostAction, TokenFactory,
};
use host::{
    aerugo::wm::types::{DecorationMode, Features, Geometry, ResizeEdge, Server, Size, ToplevelState},
    exports::aerugo::wm::wm_types::WmTypes,
};
use runner::WmRunner;
use wasmtime::{
    component::{Linker, Resource},
    Config, Engine, Store,
};

/// An ID which references an object allocated in the WM.
///
/// ID 0 is always reserved by the WM's server object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(NonZeroU32, IdType);

impl Id {
    pub fn rep(self) -> NonZeroU32 {
        self.0
    }

    pub fn ty(self) -> IdType {
        self.1
    }
}

/// The type of an id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdType {
    /// The server.
    Server,

    /// A toplevel.
    Toplevel,

    /// An output.
    Output,

    /// A snapshot.
    ///
    /// A snapshot is an object which references the contents of a surface for a given size and scale.
    Snapshot,

    /// A view is a combination of a surface and a snapshot which can be presented.
    View,
}

/// An event sent to the wm runtime.
#[derive(Debug)]
pub enum WmEvent {
    /// Notify the runtime that a new toplevel was created.
    ///
    /// This does not actually tell the wm a new toplevel was created until an initial state is sent.
    NewToplevel {
        toplevel: Id,
        features: Features,
    },

    /// Notify the runtime that a toplevel was closed.
    ClosedToplevel(Id),

    /// Notify the runtime that a toplevel's state has changed.
    UpdateToplevel {
        toplevel: Id,
        update: ToplevelUpdate,
    },

    /// Notify the runtime that a configure has been acked.
    AckToplevel {
        toplevel: Id,
        serial: u32,
    },

    NewOutput {
        output: Id,
        // TODO: Info
    },

    /// TODO: Add to wit file
    UpdateOutput {
        output: Id,
        // TODO: Info
    },

    DisconnectOutput(Id),
}

/// A request from the wm runtime.
#[derive(Debug)]
pub enum WmRequest {
    /// The display server requested the wm runtime thread terminates.
    TerminateWm,

    /// The wm runtime dropped the wm and it will no longer be used.
    ///
    /// TODO: Destruction semantics?
    ToplevelDrop(Id),

    /// The wm runtime requested the toplevel with the specified id be closed.
    ToplevelRequestClose(Id),
}

/// A message from the wm runtime.
#[derive(Debug)]
pub enum RuntimeMessage {
    Request(WmRequest),

    Closed,
}

#[derive(Debug, Clone, Default)]
pub struct ToplevelUpdate {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub min_size: ConfigureUpdate<Size>,
    pub max_size: ConfigureUpdate<Size>,
    pub geometry: ConfigureUpdate<Geometry>,
    pub parent: ConfigureUpdate<Id>,
    pub state: Option<ToplevelState>,
    pub decorations: Option<DecorationMode>,
    pub resize_edge: ConfigureUpdate<ResizeEdge>,
}

/// The WM runtime.
///
/// The wm runtime provides a communication channel with the wm. This can be registered to an event loop to
/// listen for wm requests or used to send events to the wm.
#[derive(Debug)]
#[must_use]
pub struct WmRuntime {
    channel: Channel<WmRequest>,
    sender: Sender<WmEvent>,
}

impl EventSource for WmRuntime {
    type Event = RuntimeMessage;
    type Metadata = ();
    type Ret = ();
    type Error = Box<(dyn std::error::Error + Send + Sync + 'static)>;

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        use calloop::channel;

        let mut closed = false;

        self.channel.process_events(readiness, token, |event, _| match event {
            channel::Event::Msg(request) => {
                callback(RuntimeMessage::Request(request), &mut ());
            }

            channel::Event::Closed => {
                callback(RuntimeMessage::Closed, &mut ());
                closed = true;
            }
        })?;

        // If the wm runtime thread has died or was closed then it makes no sense to continue dispatching the
        // runtime.
        if closed {
            return Ok(PostAction::Remove);
        }

        Ok(PostAction::Continue)
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> calloop::Result<()> {
        self.channel.register(poll, token_factory)
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> calloop::Result<()> {
        self.channel.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        self.channel.unregister(poll)
    }
}

impl WmRuntime {
    pub fn new(bytes: &[u8]) -> wasmtime::Result<WmRuntime> {
        let (event_sender, event_channel) = calloop::channel::channel();
        let (req_sender, req_channel) = calloop::channel::channel();

        let mut config = Config::new();
        config
            .consume_fuel(true)
            .wasm_backtrace(true)
            .wasm_component_model(true);

        let engine = Engine::new(&config)?;

        let mut store = Store::new(
            &engine,
            WmState {
                sender: req_sender,
                ids: Vec::new(),
                toplevels: HashMap::new(),
            },
        );

        let component = wasmtime::component::Component::new(&engine, bytes)?;
        let linker = Linker::new(&engine);

        // TODO: Tune the fuel amount
        store.add_fuel(10000).unwrap();

        let (aerugo_wm, instance) = host::AerugoWm::instantiate(&mut store, &component, &linker)?;
        let info = aerugo_wm
            .aerugo_wm_wm_types()
            .call_get_info(&mut store)?
            .expect("Handle string error");

        // TODO: Validate info

        // Allocate the server (id 0).
        let server = Resource::new_own(0);

        // Initialize the wm on this thread.
        let wm = aerugo_wm
            .aerugo_wm_wm_types()
            .call_create_wm(&mut store, server)?
            .expect("Handle string error");

        let mut exports = instance.exports(&mut store);
        let mut export_wm = exports.instance("wm").expect("Handle missing wm export");
        let funcs = WmTypes::new(&mut export_wm)?;

        // Rust wants us to explicitly drop exports for some reason...
        drop(exports);

        let runtime = WmRuntime {
            channel: req_channel,
            sender: event_sender,
        };

        // Start the wm thread.
        WmRunner::new(event_channel, store, wm, funcs).run()?;

        Ok(runtime)
    }
}

#[derive(Debug)]
pub enum Error {
    Id(IdError),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Id(error) => Display::fmt(error, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<IdError> for Error {
    fn from(value: IdError) -> Self {
        Error::Id(value)
    }
}

#[derive(Debug)]
pub enum IdError {
    ZeroId,

    InvalidId { rep: u32, ty: IdType },
}

impl Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::ZeroId => write!(f, "zero id"),
            IdError::InvalidId { rep, ty } => write!(f, "invalid id: Id {{ rep: {rep}, ty: {ty:?} }}"),
        }
    }
}

impl std::error::Error for IdError {}

#[derive(Debug)]
struct WmState {
    sender: Sender<WmRequest>,
    ids: Vec<Option<IdType>>,
    toplevels: HashMap<NonZeroU32, WmToplevel>,
}

impl WmState {
    fn get_id<T: 'static>(&self, resource: &Resource<T>, ty: IdType) -> Result<Id, Error> {
        let rep = NonZeroU32::new(resource.rep()).ok_or(IdError::ZeroId)?;

        if self
            .ids
            .get(rep.get() as usize)
            .ok_or(IdError::InvalidId { rep: rep.get(), ty })?
            .filter(|&info| info == ty)
            .is_none()
        {
            return Err(Error::Id(IdError::InvalidId { rep: rep.get(), ty }));
        }

        Ok(Id(rep, IdType::Toplevel))
    }

    fn validate_id_server(&self, resource: &Resource<Server>) -> Result<(), Error> {
        // The server is always assigned id 0.
        if resource.rep() != 0 {
            return Err(Error::Id(IdError::InvalidId {
                rep: resource.rep(),
                ty: IdType::Server,
            }));
        }

        Ok(())
    }

    fn get_toplevel_res<T: 'static>(&mut self, resource: &Resource<T>) -> Result<&mut WmToplevel, Error> {
        let id = self.get_id(resource, IdType::Toplevel)?;
        self.get_toplevel(id)
    }

    fn get_toplevel(&mut self, id: Id) -> Result<&mut WmToplevel, Error> {
        self.toplevels.get_mut(&id.rep()).ok_or(Error::Id(IdError::InvalidId {
            rep: id.rep().get(),
            ty: IdType::View,
        }))
    }

    fn get_toplevel_configure<T: 'static>(&self, _resource: &Resource<T>) -> Result<&mut WmToplevelConfigure, Error> {
        todo!()
    }
}

/// Toplevel wm runtime state.
#[derive(Debug)]
struct WmToplevel {
    id: Id,
    initial_commit: bool,
    features: Features,
    app_id: Option<String>,
    title: Option<String>,
    min_size: Option<Size>,
    max_size: Option<Size>,
    geometry: Option<Geometry>,
    parent: Option<Id>,
    state: ToplevelState,
    decorations: DecorationMode,
    resize_edge: Option<ResizeEdge>,
}

#[derive(Debug, Clone, Default)]
pub enum ConfigureUpdate<T> {
    #[default]
    None,
    Update(Option<T>),
}

impl<T> ConfigureUpdate<T> {
    pub fn is_update(&self) -> bool {
        matches!(self, Self::Update(_))
    }
}

#[derive(Debug)]
struct WmToplevelConfigure {
    toplevel_id: Id,
    decorations: Option<DecorationMode>,
    parent: ConfigureUpdate<Id>,
    state: Option<ToplevelState>,
    size: ConfigureUpdate<Size>,
    bounds: ConfigureUpdate<Size>,
}

#[cfg(test)]
mod tests {
    use crate::{Id, WmEvent, WmRequest};

    fn assert_send<T: Send>() {}

    #[test]
    fn is_id_send() {
        assert_send::<Id>();
    }

    #[test]
    fn is_event_send() {
        assert_send::<WmEvent>();
    }

    #[test]
    fn is_request_send() {
        assert_send::<WmRequest>();
    }
}
