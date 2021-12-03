use std::{env, error::Error};

use slog::{info, Logger};
use smithay::{
    backend::{
        self,
        drm::DrmNode,
        egl::{EGLContext, EGLDisplay},
        x11::{Window, X11Event, X11Surface},
    },
    reexports::{calloop::LoopHandle, gbm, wayland_server::Display},
};

use crate::state::NameMe;

use super::Backend;

#[derive(Debug)]
pub struct X11Backend {
    logger: Logger,
    // TODO: Replace this with X11Handle when PR is merged.
    window: Option<Window>,
    outputs: Vec<X11Output>,
    // TODO: Replace this with the mutex we use for the device?
    gbm_device: Option<gbm::Device<DrmNode>>,

    // TODO: Vulkan in the future
    #[allow(dead_code)]
    egl_display: EGLDisplay,
    egl_context: EGLContext,
}

impl Backend for X11Backend {
    fn new(logger: Logger, handle: LoopHandle<'_, NameMe>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let backend = backend::x11::X11Backend::new(logger.clone())?;
        let window = backend.window();
        let logger = logger.new(slog::o!("backend" => "x11"));

        // Setup the renderer
        let drm_node = backend.drm_node()?;
        let gbm_device = gbm::Device::new(drm_node)?;
        // EGL init
        let egl_display = EGLDisplay::new(&gbm_device, logger.clone())?;
        let egl_context = EGLContext::new(&egl_display, logger.clone())?;

        handle.insert_source(backend, handle_backend_event)?;

        Ok(X11Backend {
            logger,
            // TODO: Replace with X11Handle
            window: Some(window),
            outputs: vec![],
            gbm_device: Some(gbm_device),
            egl_display,
            egl_context,
        })
    }

    fn available() -> bool
    where
        Self: Sized,
    {
        env::var("DISPLAY").is_ok()
    }

    fn name(&self) -> &str {
        "x11"
    }

    fn logger(&self) -> &Logger {
        &self.logger
    }

    fn setup_outputs(&mut self, _display: &mut Display) -> Result<(), Box<dyn Error>> {
        // We start with one window.
        // TODO: Create window when multi-window is merged
        let window = self.window.take().unwrap();
        // TODO: Lock the gbm device mutex when multi-window is merged.
        let gbm_device = self.gbm_device.take().unwrap();

        let surface = X11Surface::new(
            todo!(),
            gbm_device,
            self.egl_context
                .dmabuf_texture_formats()
                .iter()
                .map(|format| format.modifier),
        )?;

        // Create the output
        let output = X11Output { window, surface };

        self.outputs.push(output);
    }
}

#[derive(Debug)]
struct X11Output {
    window: Window,
    #[allow(dead_code)]
    surface: X11Surface,
}

impl Drop for X11Output {
    fn drop(&mut self) {
        // TODO: Destroy the global?
    }
}

/// Handler for events dispatched by the X11 backend.
fn handle_backend_event(event: X11Event, window: &mut Window, name_me: &mut NameMe) {
    match event {
        X11Event::CloseRequested => {
            let backend = name_me.state.downcast_backend_mut::<X11Backend>().unwrap();

            // Destroy the output for the window that has been closed.
            // TODO: Wait does the window get closed actually?
            backend.outputs.retain(|output| &output.window != window);

            if backend.outputs.is_empty() {
                info!(
                    name_me.state.backend().logger(),
                    "Quitting because all outputs are destroyed"
                );
                name_me.state.continue_loop = false;
            }
        }

        X11Event::Input(event) => name_me.state.handle_input(event),

        _ => (),
    }
}
