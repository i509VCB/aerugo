//! Wasm WM runtime for the Aerugo.

mod host;

use std::{
    collections::HashMap,
    fmt::{self, Display},
    num::NonZeroU32,
    thread,
};

use calloop::{
    channel::{Channel, Sender},
    EventSource, Poll, PostAction, TokenFactory,
};
use host::aerugo::wm::types::{
    DecorationMode, Features, Geometry, ResizeEdge, Server, Size, ToplevelId, ToplevelState,
};
use wasmtime::{component::Resource, Config, Engine, Store};

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
    Server,
    Toplevel,
    Output,
    Image,
    Node,
}

/// An event sent to the wm runtime.
#[derive(Debug)]
pub enum WmEvent {}

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
pub enum RuntimeEvent {
    Request(WmRequest),

    Closed,
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
    type Event = RuntimeEvent;
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
                callback(RuntimeEvent::Request(request), &mut ());
            }

            channel::Event::Closed => {
                callback(RuntimeEvent::Closed, &mut ());
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
    pub fn new(_bytes: &[u8]) -> wasmtime::Result<WmRuntime> {
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

        // TODO: Tune the fuel amount
        store.add_fuel(10000).unwrap();

        // Initialize the wm on this thread.
        // TODO

        let runtime = WmRuntime {
            channel: req_channel,
            sender: event_sender,
        };

        // Start the wm thread.
        let _ = thread::Builder::new().name("aerugo wm runtime".into()).spawn(move || {
            let thread = WmThread {
                channel: event_channel,
                store,
            };

            loop {
                // Since this is run on a separate thread, we want to manually poll and suspend the thread if no
                // wm events are pending.
                match thread.channel.recv() {
                    Ok(event) => {
                        // Dispatch the event on the runtime.
                        // Add some fuel for while dispatching.
                        let _ = event;
                    }

                    // The other end was closed.
                    Err(_) => return,
                }
            }
        })?;

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
struct WmThread {
    channel: Channel<WmEvent>,
    store: Store<WmState>,
}

#[derive(Debug)]
struct WmState {
    sender: Sender<WmRequest>,
    ids: Vec<Option<IdType>>,
    toplevels: HashMap<NonZeroU32, WmToplevel>,
}

impl WmState {
    fn validate_id<T: 'static>(&self, resource: &Resource<T>, ty: IdType) -> Result<NonZeroU32, Error> {
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

        Ok(rep)
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

    fn get_toplevel<T: 'static>(&self, resource: &Resource<T>) -> Result<&WmToplevel, Error> {
        let rep = self.validate_id(resource, IdType::Toplevel)?;
        self.toplevels.get(&rep).ok_or(Error::Id(IdError::InvalidId {
            rep: rep.get(),
            ty: IdType::Node,
        }))
    }
}

/// Toplevel wm runtime state.
#[derive(Debug)]
struct WmToplevel {
    id: Id,
    features: Features,
    app_id: Option<String>,
    title: Option<String>,
    min_size: Option<Size>,
    max_size: Option<Size>,
    geometry: Option<Geometry>,
    parent: Option<ToplevelId>,
    state: ToplevelState,
    decorations: DecorationMode,
    resize_edge: Option<ResizeEdge>,
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
