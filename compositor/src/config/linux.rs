use std::{
    fs, io,
    os::unix::prelude::AsRawFd,
    path::{Path, PathBuf},
};

use inotify::{EventMask, EventOwned, Inotify, WatchDescriptor, WatchMask};
use slog::{info, Logger};
use smithay::reexports::calloop::{EventSource, Interest, Mode, Poll, PostAction, Readiness, Token, TokenFactory};

use super::watcher;

#[derive(Debug)]
pub struct InotifySource {
    inotify: Inotify,
    token: Token,
    watch: WatchDescriptor,
    path: PathBuf,
    _logger: Logger,
}

impl InotifySource {
    pub fn new<L>(path: &(impl AsRef<Path> + ?Sized), logger: L) -> io::Result<InotifySource>
    where
        L: Into<Option<slog::Logger>>,
    {
        let logger = logger.into().unwrap_or_else(|| Logger::root(slog::Discard, slog::o!()));
        let path = path.as_ref().to_owned();

        // Make sure the path to the directory we are watching exists.
        fs::create_dir_all(&path)?;

        let mut inotify = Inotify::init()?;
        let watch = inotify.add_watch(&path, WatchMask::all())?;

        let logger = logger.new(slog::o!(
            "watcher" => "inotify",
            "path" => path.display().to_string(),
        ));

        info!(logger, "Initialized watcher");

        Ok(InotifySource {
            inotify,
            token: Token::invalid(),
            watch,
            path,
            _logger: logger,
        })
    }
}

impl EventSource for InotifySource {
    type Event = EventOwned;

    /// The directory which is being watched for file changes.
    type Metadata = PathBuf;

    type Ret = ();

    fn process_events<F>(&mut self, _readiness: Readiness, token: Token, mut callback: F) -> std::io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        if token == self.token {
            let mut buffer = [0; 4096];

            slog::error!(self._logger, "Dispatch");

            for event in self.inotify.read_events(&mut buffer)? {
                if event.wd == self.watch {
                    // Always clone the path so users can not modify it.
                    callback(event.into_owned(), &mut self.path.clone());
                }
            }

            assert_eq!(self.inotify.read_events(&mut buffer)?.count(), 0, "Not empty");
        }

        Ok(PostAction::Continue)
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        let token = token_factory.token();
        poll.register(self.inotify.as_raw_fd(), Interest::READ, Mode::Level, token)?;
        self.token = token;

        Ok(())
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        let token = token_factory.token();
        poll.reregister(self.inotify.as_raw_fd(), Interest::READ, Mode::Level, token)?;
        self.token = token;

        Ok(())
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.token = Token::invalid();
        poll.unregister(self.inotify.as_raw_fd())
    }
}

// Abstracted event source

#[derive(Debug)]
pub(crate) struct PlatformEventSource {
    inner: InotifySource,
}

impl PlatformEventSource {
    pub fn new<L>(path: &(impl AsRef<Path> + ?Sized), logger: L) -> io::Result<PlatformEventSource>
    where
        L: Into<Option<slog::Logger>>,
    {
        Ok(PlatformEventSource {
            inner: InotifySource::new(path, logger)?,
        })
    }
}

impl EventSource for PlatformEventSource {
    type Event = watcher::Event;

    /// The directory which is being watched for file changes.
    type Metadata = PathBuf;

    type Ret = ();

    fn process_events<F>(&mut self, readiness: Readiness, token: Token, mut callback: F) -> std::io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.inner.process_events(readiness, token, |event, path| {
            if event.mask.contains(EventMask::CREATE) || event.mask.contains(EventMask::MOVED_TO) {
                let mut created = path.clone();
                created.push(event.name.unwrap());

                callback(watcher::Event::Created(created), path)
            } else if event.mask.contains(EventMask::DELETE) || event.mask.contains(EventMask::MOVED_FROM) {
                let mut removed = path.clone();
                removed.push(event.name.unwrap());

                callback(watcher::Event::Removed(removed), path)
            } else if event.mask.contains(EventMask::MODIFY) {
                let mut modified = path.clone();
                modified.push(event.name.unwrap());

                callback(watcher::Event::Modified(modified), path)
            }
        })
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.inner.register(poll, token_factory)
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> io::Result<()> {
        self.inner.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.inner.unregister(poll)
    }
}
