use std::{
    fmt, fs, io,
    os::unix::prelude::{AsRawFd, RawFd},
    path::PathBuf,
};

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use slog::{debug, info, Logger};

use super::watcher;

pub struct WatcherInner {
    inotify: Inotify,
    path: PathBuf,
    watch: WatchDescriptor,
    logger: Logger,
}

impl fmt::Debug for WatcherInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: See debug field content
        f.debug_struct("WatcherInner")
            .field(
                "inotify",
                &"Pending PR: https://github.com/hannobraun/inotify-rs/pull/180",
            )
            .finish()
    }
}

impl AsRawFd for WatcherInner {
    fn as_raw_fd(&self) -> RawFd {
        self.inotify.as_raw_fd()
    }
}

impl WatcherInner {
    pub fn new(watching: PathBuf, logger: Logger) -> io::Result<WatcherInner> {
        // Make sure the path to the directory we are watching exists.
        fs::create_dir_all(&watching)?;

        let mut inotify = Inotify::init()?;
        let path = &watching.to_string_lossy().into_owned();
        let watch = inotify.add_watch(
            &watching,
            WatchMask::CREATE
                | WatchMask::DELETE
                | WatchMask::MODIFY
                | WatchMask::MOVED_FROM
                | WatchMask::MOVED_TO,
        )?;

        let logger = logger.new(slog::o!(
            "wayland_compositor" => "inotify_config_watcher",
            "path" => path.clone()
        ));

        info!(logger, "Initialized watcher");

        Ok(WatcherInner {
            inotify,
            path: watching,
            watch,
            logger,
        })
    }

    pub fn read_events<F>(&mut self, mut f: F) -> io::Result<()>
    where
        F: FnMut(watcher::Event),
    {
        let mut buffer = [0; 1024];

        for event in self.inotify.read_events(&mut buffer)? {
            if event.wd == self.watch {
                if event.mask.contains(EventMask::CREATE)
                    || event.mask.contains(EventMask::MOVED_TO)
                {
                    let mut path = self.path.clone();
                    path.push(event.name.unwrap().to_owned());

                    debug!(
                        self.logger,
                        "Created file";
                        "file" => &path.file_name().unwrap().to_string_lossy().into_owned()
                    );
                    f(watcher::Event::Created(path))
                } else if event.mask.contains(EventMask::DELETE)
                    || event.mask.contains(EventMask::MOVED_FROM)
                {
                    let mut path = self.path.clone();
                    path.push(event.name.unwrap().to_owned());

                    debug!(
                        self.logger,
                        "Removed file";
                        "file" => &path.file_name().unwrap().to_string_lossy().into_owned()
                    );
                    f(watcher::Event::Removed(path))
                } else if event.mask.contains(EventMask::MODIFY) {
                    let mut path = self.path.clone();
                    path.push(event.name.unwrap().to_owned());

                    debug!(
                        self.logger,
                        "File modified";
                        "file" => &path.file_name().unwrap().to_string_lossy().into_owned()
                    );
                    f(watcher::Event::Modified(path))
                }
            }
        }

        Ok(())
    }
}
