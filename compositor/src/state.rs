use slog::{error, info, Logger};
use smithay::{
    backend::input::{InputBackend, InputEvent},
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_server::Display,
    },
    wayland::{compositor::compositor_init, data_device, shm::init_shm_global},
};
use std::{cell::RefCell, error::Error, rc::Rc, time::Duration};

use crate::{backend::Backend, shell::Shell, CreateBackendFn};

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

#[warn(missing_docs)] // Use of these functions should be explicitly known.
impl State {
    /// Returns a new instance of the compositor state.
    ///
    /// This function takes another function used to instantiate the backend the compositor should use.
    pub fn new(
        logger: Logger,
        loop_handle: LoopHandle<'static, State>,
        display: Rc<RefCell<Display>>,
        socket_name: Socket,
        backend: CreateBackendFn,
    ) -> Result<State, Box<dyn Error>> {
        let (backend, shell) = {
            let display = &mut *display.borrow_mut();

            insert_wayland_source(loop_handle.clone(), display)?;
            // Initialize compositor globals
            setup_globals(display, logger.clone())?;

            let backend = backend(logger.clone(), loop_handle.clone(), display)?;

            // Initialize the shell, in our case the XDG and Layer shell
            (backend, Shell::new(display, logger.clone())?)
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

    /// Returns the name of the socket this compositor is exposed at.
    pub fn socket_name(&self) -> Option<&str> {
        self.socket_name.as_ref().map(|s| s as &str)
    }

    /// Returns a dynamically typed reference to the backend in use.
    ///
    /// Most compositor logic should use the dynamically typed functions to get access to some of the state the backend
    /// manages.
    ///
    /// Backend implementations may use [`State::downcast_backend`] to access internal data.
    pub fn backend(&self) -> &dyn Backend {
        self.backend.as_ref()
    }

    /// Returns a dynamically typed unique reference to the backend in use.
    ///
    /// Most compositor logic should use the dynamically typed functions to mutate the state the backend.
    ///
    /// Backend implementations may use [`State::downcast_backend_mut`] to mutate internal data.
    pub fn backend_mut(&mut self) -> &mut dyn Backend {
        self.backend.as_mut()
    }

    /// Returns a reference to the backend in use by the compositor.
    ///
    /// You must know the type of the backend ahead of time in order to resolve a reference to the backend.
    pub fn downcast_backend<B: Backend>(&self) -> Option<&B> {
        self.backend.downcast_ref()
    }

    /// Returns a unique to the backend in use by the compositor.
    ///
    /// You must know the type of the backend ahead of time in order to resolve a unique reference to the backend.
    pub fn downcast_backend_mut<B: Backend>(&mut self) -> Option<&mut B> {
        self.backend.downcast_mut()
    }

    /// Returns a reference to the objects managed by the compositor's shell.
    pub fn shell(&self) -> &Shell {
        &self.shell
    }

    /// Returns a unique reference to the objects managed by the compositor's shell.
    pub fn shell_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }

    /// Returns `true` if the compositor's event loop should shut down the compositor.
    pub fn should_continue(&mut self) -> bool {
        if !self.continue_loop {
            return false;
        }

        true
    }

    /// Handles some input event.
    ///
    /// Any special events, as defined by the [`InputBackend::Special`] variant should not be passed into this function
    /// and instead handled in the originating backend before calling this function
    pub fn handle_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        #[allow(clippy::single_match)] // temporary
        match event {
            InputEvent::Special(_) => unreachable!("special event encountered in common input handler"),
            // Not implemented yet
            _ => (),
        }
    }
}

fn insert_wayland_source(handle: LoopHandle<'static, State>, display: &Display) -> Result<(), Box<dyn Error>> {
    handle.insert_source(
        Generic::from_fd(
            // The file descriptor which indicates there are pending messages
            display.get_poll_fd(),
            Interest::READ,
            Mode::Level,
        ),
        move |_, _, state: &mut State| {
            let display = state.display.clone();
            let mut display = display.borrow_mut();

            if let Err(err) = display.dispatch(Duration::ZERO, state) {
                error!(state.logger, "Error while dispatching requests"; "error" => &err);
                state.continue_loop = false;
                Err(err)
            } else {
                Ok(PostAction::Continue)
            }
        },
    )?;

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

    data_device::init_data_device(display, |_| {}, data_device::default_action_chooser, logger);

    Ok(())
}
