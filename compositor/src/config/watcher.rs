use std::{
    io,
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use slog::Logger;
use smithay::reexports::calloop::{
    EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory,
};

#[cfg(target_os = "linux")]
pub use crate::config::linux::*;

#[cfg(not(target_os = "linux"))]
compile_error!("No config watcher implementation outside of linux at the moment.");

#[derive(Debug)]
pub struct DirWatcher {
    inner: WatcherInner,
}

impl DirWatcher {
    pub fn new(watching: &(impl AsRef<Path> + ?Sized), logger: Logger) -> io::Result<DirWatcher> {
        let path = watching.as_ref().to_owned();

        let inner = WatcherInner::new(path, logger)?;

        Ok(DirWatcher { inner })
    }
}

impl EventSource for DirWatcher {
    type Event = Event;

    type Metadata = ();

    type Ret = ();

    fn process_events<F>(
        &mut self,
        _readiness: Readiness,
        _token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.inner.read_events(|event| {
            // We clone the path here so callbacks cannot change the path.
            callback(event, &mut ())
        })?;

        Ok(PostAction::Continue)
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        poll.register(
            self.inner.as_raw_fd(),
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        poll.reregister(
            self.inner.as_raw_fd(),
            Interest::READ,
            Mode::Level,
            token_factory.token(),
        )
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        poll.unregister(self.inner.as_raw_fd())
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Event {
    /// The file has been created.
    Created(PathBuf),

    /// The file has been modified.
    Modified(PathBuf),

    /// The file has been removed.
    Removed(PathBuf),
}
