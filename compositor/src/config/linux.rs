use std::{
    fs, io,
    path::PathBuf,
    thread::{self, JoinHandle},
    time::Duration,
};

use inotify::{EventMask, Inotify, WatchMask};
use slog::{debug, error, info, Logger};
use smithay::reexports::calloop::{self, channel::Channel};

use super::watcher;

pub fn start_watcher(watching: PathBuf, logger: Logger) -> io::Result<(Channel<watcher::Event>, JoinHandle<()>)> {
    // Make sure the path to the directory we are watching exists.
    fs::create_dir_all(&watching)?;

    let (sender, channel) = calloop::channel::channel();

    let mut inotify = Inotify::init()?;
    let path = &watching.to_string_lossy().into_owned();
    let watch = inotify.add_watch(
        &watching,
        WatchMask::CREATE | WatchMask::DELETE | WatchMask::MODIFY | WatchMask::MOVED_FROM | WatchMask::MOVED_TO,
    )?;

    let logger = logger.new(slog::o!(
        "watcher" => "inotify",
        "path" => path.clone()
    ));

    info!(logger, "Initialized watcher");

    let watch_thread = thread::spawn(move || {
        let logger = logger.clone();
        let mut buffer = [0; 4096];

        loop {
            // Shutdown
            if sender.send(watcher::Event::ThreadWakeup).is_err() {
                break;
            }

            let mut channel_closed = false;

            match inotify.read_events(&mut buffer) {
                Ok(events) => {
                    for event in events {
                        if event.wd == watch {
                            channel_closed =
                                if event.mask.contains(EventMask::CREATE) || event.mask.contains(EventMask::MOVED_TO) {
                                    let mut path = watching.clone();
                                    path.push(event.name.unwrap());

                                    debug!(
                                        logger,
                                        "Created dir entry";
                                        "entry" => &path.file_name().unwrap().to_string_lossy().into_owned()
                                    );

                                    sender.send(watcher::Event::Created(path))
                                } else if event.mask.contains(EventMask::DELETE)
                                    || event.mask.contains(EventMask::MOVED_FROM)
                                {
                                    let mut path = watching.clone();
                                    path.push(event.name.unwrap());

                                    debug!(
                                        logger,
                                        "Removed dir entry";
                                        "entry" => &path.file_name().unwrap().to_string_lossy().into_owned()
                                    );

                                    sender.send(watcher::Event::Removed(path))
                                } else if event.mask.contains(EventMask::MODIFY) {
                                    let mut path = watching.clone();
                                    path.push(event.name.unwrap());

                                    debug!(
                                        logger,
                                        "Entry modified";
                                        "modified" => &path.file_name().unwrap().to_string_lossy().into_owned()
                                    );

                                    sender.send(watcher::Event::Modified(path))
                                } else {
                                    Ok(())
                                }
                                .is_err();
                        }

                        if channel_closed {
                            break;
                        }
                    }
                }

                Err(err) => {
                    error!(logger, "Error while reading events {}", err);
                    break;
                }
            }

            if channel_closed {
                break;
            }

            // Park the test thread for a little time to not burn cpus
            thread::park_timeout(Duration::from_secs(2));
        }
    });

    Ok((channel, watch_thread))
}
