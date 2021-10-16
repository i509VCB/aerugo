use directories::ProjectDirs;
use slog::{info, Logger};
use smithay::{
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_server::Display,
    },
    wayland::{compositor::compositor_init, shm::init_shm_global},
};
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use crate::{backend::Backend, config::watcher::DirWatcher, shell::Shell};

#[derive(Debug)]
pub enum Socket {
    /// Do not add a socket.
    None,

    /// Add a socket using an automatic name.
    Auto,

    /// Add a socket using the specified name.
    Named(String),
}

#[derive(Debug)]
pub struct State {
    pub logger: Logger,
    pub display: Rc<RefCell<Display>>,
    pub continue_loop: bool,
    socket_name: Option<String>,
    backend: Box<dyn Backend>,
    shell: Shell,
}

impl State {
    pub fn new(
        logger: Logger,
        loop_handle: LoopHandle<'static, State>,
        display: Rc<RefCell<Display>>,
        socket_name: Socket,
        backend: impl Backend + 'static,
    ) -> Result<State, Box<dyn Error>> {
        let mut backend = Box::new(backend);

        let shell = {
            let display = &mut *display.borrow_mut();

            insert_wayland_source(loop_handle.clone(), display)?;
            backend.setup_backend(loop_handle.clone())?;
            setup_dir_watcher(loop_handle.clone(), logger.clone())?;

            // Initialize compositor globals
            setup_globals(display, logger.clone())?;

            // Setup any backend originating globals, such as wl_drm and outputs.
            backend.setup_globals(display)?;

            // Initialize the shell, in our case the XDG and Layer shell
            Shell::new(display, logger.clone())?
        };

        info!(logger, r#"Starting with backend "{backend}""#, backend = backend.name());

        let socket_name = {
            match socket_name {
                Socket::None => None,

                Socket::Auto => Some(
                    display
                        .borrow_mut()
                        .add_socket_auto()?
                        .into_string()
                        .expect("Wayland socket name was not a Rust string"),
                ),

                Socket::Named(socket_name) => {
                    display.borrow_mut().add_socket(Some(socket_name.clone()))?;
                    Some(socket_name)
                }
            }
        };

        if let Some(socket_name) = &socket_name {
            info!(logger, "Listening on wayland socket"; "name" => socket_name);
        }

        Ok(State {
            logger,
            display,
            continue_loop: true,
            socket_name,
            backend,
            shell,
        })
    }

    pub fn socket_name(&self) -> Option<&str> {
        self.socket_name.as_ref().map(|s| s as &str)
    }

    pub fn backend(&self) -> &dyn Backend {
        self.backend.as_ref()
    }

    pub fn backend_mut(&mut self) -> &mut dyn Backend {
        self.backend.as_mut()
    }

    pub fn shell(&self) -> &Shell {
        &self.shell
    }

    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }

    pub fn should_continue(&mut self) -> bool {
        if !self.continue_loop {
            return false;
        }

        true
    }
}

/// Inserts a Wayland source into the loop.
fn insert_wayland_source(handle: LoopHandle<'static, State>, display: &Display) -> Result<(), Box<dyn Error>> {
    handle.insert_source(
        Generic::from_fd(
            display.get_poll_fd(), // The file descriptor which indicates there are pending messages
            Interest::READ,
            Mode::Level,
        ),
        // This callback is invoked when the poll file descriptor has had activity, indicating there are pending messages.
        move |_, _, state: &mut State| {
            let display = state.display.clone();
            let mut display = display.borrow_mut();

            if let Err(e) = display.dispatch(Duration::ZERO, state) {
                state.continue_loop = false;
                Err(e)
            } else {
                Ok(PostAction::Continue)
            }
        },
    )?;

    Ok(())
}

fn setup_dir_watcher(handle: LoopHandle<'static, State>, logger: Logger) -> Result<(), Box<dyn Error>> {
    let project_dirs = ProjectDirs::from("", "i5", "wayland_compositor").unwrap();

    let config_dir = project_dirs.config_dir();

    let watcher = DirWatcher::new(config_dir, logger)?;

    handle.insert_source(watcher, |event, _, _state| {
        println!("{:?}", event);
    })?;

    Ok(())
}

fn setup_globals(display: &mut Display, logger: Logger) -> Result<(), Box<dyn Error>> {
    // TODO: Should we offer any additional formats for the shm global?
    init_shm_global(display, vec![], logger.clone());

    compositor_init(
        display,
        move |surface, mut ddata| {
            ddata.get::<State>().unwrap().handle_surface_commit(&surface);
        },
        logger.clone(),
    );

    Ok(())
}
