use std::{
    io,
    path::{Path, PathBuf},
};

use slog::Logger;
use smithay::reexports::calloop::{EventSource, Poll, PostAction, Readiness, Token, TokenFactory};

use crate::config::imp::*;

#[derive(Debug)]
pub struct DirWatcher {
    inner: PlatformEventSource,
}

impl DirWatcher {
    pub fn new(path: &(impl AsRef<Path> + ?Sized), logger: Logger) -> io::Result<DirWatcher> {
        Ok(DirWatcher {
            inner: PlatformEventSource::new(path, logger)?,
        })
    }
}

impl EventSource for DirWatcher {
    type Event = Event;

    /// The directory which is being watched for file changes.
    type Metadata = PathBuf;

    type Ret = ();

    fn process_events<F>(&mut self, readiness: Readiness, token: Token, mut callback: F) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.inner
            .process_events(readiness, token, |event, path| callback(event, path))
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

#[cfg(test)]
mod test {
    use std::{
        env,
        fs::{self, File},
        io,
        time::Duration,
    };

    use slog::{Drain, Logger};
    use smithay::reexports::calloop::EventLoop;

    use super::DirWatcher;

    #[test]
    fn test_watcher() -> io::Result<()> {
        struct TestState {
            created: bool,
            modified: bool,
            deleted: bool,
        }

        let mut state = TestState {
            created: false,
            modified: false,
            deleted: false,
        };

        // Initialize logger
        let logger = Logger::root(
            slog_async::Async::default(slog_term::term_full().fuse()).fuse(),
            slog::o!(),
        );

        let _guard = slog_scope::set_global_logger(logger.clone());
        slog_stdlog::init().expect("Could not setup log backend");

        let mut event_loop = EventLoop::<TestState>::try_new().unwrap();
        let mut test_dir = env::temp_dir();
        test_dir.push("test_watcher");

        // Clear the directory for testing if anything exists
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir)?;

        let watcher = DirWatcher::new(&test_dir, logger).expect("Watcher not created");

        event_loop
            .handle()
            .insert_source(watcher, |event, _, state| match event {
                crate::config::watcher::Event::Created(_) => {
                    state.created = true;
                }
                crate::config::watcher::Event::Modified(_) => {
                    state.modified = true;
                }
                crate::config::watcher::Event::Removed(_) => {
                    state.deleted = true;
                }
            })
            .unwrap();

        // Dispatch once to set up.
        event_loop.dispatch(Duration::from_millis(0), &mut state)?;

        // Create a file
        let mut test = test_dir.clone();
        test.push("test.txt");
        let _ = File::create(&test)?;

        event_loop.dispatch(Duration::from_millis(0), &mut state)?;

        assert_eq!(state.created, true, "File creation not detected");

        // Write to the file
        // {
        //     let mut file = File::create(&test)?;
        //     file.write_all(b"Test file contents")?;
        //     file.flush()?;
        // }

        // event_loop.dispatch(Duration::from_millis(200), &mut state)?;

        // assert_eq!(state.modified, true, "File modification not detected");

        // Delete the file
        fs::remove_file(test)?;

        // Let's be extremely generous with the amount of time we allow platforms to respond in.
        event_loop.dispatch(Duration::from_secs(10), &mut state)?;

        assert_eq!(state.deleted, true, "Deletion not detected");

        Ok(())
    }
}
