use slog::{info, Logger};
use smithay::{
    backend::input::{InputBackend, InputEvent},
    reexports::wayland_server::Display,
    wayland::{compositor::compositor_init, data_device, shm::init_shm_global},
};
use std::error::Error;

use crate::backend::Backend;

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
pub struct NameMe {
    pub display: Display,
    pub state: CompositorState,
}

#[derive(Debug)]
pub struct CompositorState {
    pub logger: Logger,
    pub continue_loop: bool,
    socket_name: Option<String>,
    backend: Box<dyn Backend>,
}

// Use of these functions should be explicitly known.
#[warn(missing_docs)]
impl CompositorState {
    /// Returns a new instance of the compositor state.
    ///
    /// This function takes another function used to instantiate the backend the compositor should use.
    pub fn new(
        logger: Logger,
        display: &mut Display,
        socket_name: Socket,
        backend: Box<dyn Backend + '_>,
    ) -> Result<CompositorState, Box<dyn Error>> {
        info!(logger, r#"Starting with backend "{backend}""#, backend = backend.name());

        setup_globals(display, logger.clone())?;

        let socket_name = {
            match socket_name {
                Socket::None => None,

                Socket::Auto => Some(
                    display
                        .add_socket_auto()?
                        .into_string()
                        .expect("Wayland socket name was not a Rust string"),
                ),

                Socket::Named(socket_name) => {
                    display.add_socket(Some(socket_name.clone()))?;
                    Some(socket_name)
                }
            }
        };

        if let Some(socket_name) = &socket_name {
            info!(logger, "Listening on wayland socket"; "name" => socket_name);
        }

        Ok(CompositorState {
            logger,
            continue_loop: true,
            socket_name,
            backend,
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
    pub fn backend(&self) -> &dyn Backend {
        self.backend.as_ref()
    }

    /// Returns a dynamically typed unique reference to the backend in use.
    ///
    /// Most compositor logic should use the dynamically typed functions to mutate the state the backend.
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

    /// Returns `true` if the compositor's event loop should shut down the compositor.
    pub fn should_continue(&mut self) -> bool {
        if !self.continue_loop {
            return false;
        }

        true
    }

    /// Handles some input event.
    ///
    /// Any special events, as defined by the [`InputEvent::Special`] variant should not be passed into this function
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

fn setup_globals(display: &mut Display, logger: Logger) -> Result<(), Box<dyn Error>> {
    // TODO: Should we offer any additional formats for the shm global?
    init_shm_global(display, vec![], logger.clone());

    compositor_init(
        display,
        move |_surface, mut _ddata| {
            todo!()
            //ddata.get::<CompositorState>().unwrap().handle_surface_commit(&surface);
        },
        logger.clone(),
    );

    data_device::init_data_device(display, |_| {}, data_device::default_action_chooser, logger);

    Ok(())
}
