use std::{
    io,
    path::{Path, PathBuf},
    thread::JoinHandle,
};

use slog::Logger;
use smithay::reexports::calloop::{
    channel::{self, sync_channel, Channel},
    EventSource, Poll, PostAction, Readiness, Token, TokenFactory,
};

use crate::config::imp::*;

#[derive(Debug)]
pub struct DirWatcher {
    channel: Channel<Event>,
    watch_thread: JoinHandle<()>,
    logger: Logger,
}

impl DirWatcher {
    pub fn new(watching: &(impl AsRef<Path> + ?Sized), logger: Logger) -> io::Result<DirWatcher> {
        let (channel, watch_thread) = start_watcher(watching.as_ref().to_owned(), logger.clone())?;

        Ok(DirWatcher {
            channel,
            watch_thread,
            logger,
        })
    }
}

impl Drop for DirWatcher {
    fn drop(&mut self) {
        {
            // Signal the worker thread to exit by dropping the read end of the channel.
            // There is no easy and nice way to do this, so do it the ugly way: Replace it.
            let (_, channel) = sync_channel(1);
            self.channel = channel;
        }

        // Unpark the thread to instantly shut down
        self.watch_thread.thread().unpark();
    }
}

impl EventSource for DirWatcher {
    type Event = Event;

    type Metadata = ();

    type Ret = ();

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.channel
            .process_events(readiness, token, |event, _| match event {
                channel::Event::Msg(event) => {
                    if let Event::ThreadWakeup = event {
                    } else {
                        callback(event, &mut ());
                    }
                }

                channel::Event::Closed => (),
            })
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.channel.register(poll, token_factory)
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.channel.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.channel.unregister(poll)
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

    /// The watch thread was waken up and will check for changes.
    ///
    /// This event is never exposed to users and is, only used internally.
    #[doc(hidden)]
    ThreadWakeup,
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use directories::ProjectDirs;
    use slog::{Drain, Logger};
    use smithay::reexports::calloop::EventLoop;

    use super::DirWatcher;

    #[test]
    fn test_watcher() {
        // Initialize logger
        let logger = Logger::root(
            slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
            slog::o!(),
        );

        let _guard = slog_scope::set_global_logger(logger.clone());
        slog_stdlog::init().expect("Could not setup log backend");

        let mut event_loop = EventLoop::<()>::try_new().unwrap();
        let project_dirs = ProjectDirs::from("", "i5", "wayland_compositor").unwrap();
        let config_dir = project_dirs.config_dir();

        let watcher = DirWatcher::new(config_dir, logger).expect("Watcher not created");

        event_loop
            .handle()
            .insert_source(watcher, |_event, _, _| {})
            .unwrap();

        event_loop
            .run(Duration::from_millis(10), &mut (), |_| {})
            .expect("Failed to run event loop")
    }
}
