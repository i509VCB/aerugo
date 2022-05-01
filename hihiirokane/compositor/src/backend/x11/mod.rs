use std::{
    env,
    error::Error,
    sync::{Arc, Mutex},
};

use slog::{info, Logger};
use smithay::{
    backend::{
        self, allocator,
        egl::{EGLContext, EGLDisplay},
        input::{InputEvent, KeyState, KeyboardKeyEvent},
        renderer::{gles2::Gles2Renderer, Bind, Frame, Renderer, Unbind},
        x11::{Window, WindowBuilder, X11Event, X11Handle, X11Surface},
    },
    reexports::{
        calloop::{generic::Fd, LoopHandle},
        gbm,
        wayland_server::Display,
    },
    utils::{Rectangle, Transform},
};

use crate::state::NameMe;

use super::Backend;

#[derive(Debug)]
pub struct X11Backend {
    logger: Logger,
    handle: X11Handle,
    outputs: Vec<X11Output>,
    formats: Vec<allocator::Modifier>,

    // TODO: Vulkan in the future
    renderer: Gles2Renderer,
    _egl_display: EGLDisplay,
    // The native display type must outlive everything created by EGL. Even though the display dropping before
    // this is fine, it is not ideal to have this dropped before the display.
    gbm_device: Arc<Mutex<gbm::Device<Fd>>>,
}

impl Backend for X11Backend {
    fn new(logger: Logger, handle: LoopHandle<'_, NameMe>, _display: &mut Display) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let backend = backend::x11::X11Backend::new(logger.clone())?;
        let x_handle = backend.handle();
        let logger = logger.new(slog::o!("backend" => "x11"));

        // Setup the renderer
        let gbm_device = gbm::Device::new(Fd(x_handle.drm_node()?.1))?;
        // EGL init
        let egl_display = EGLDisplay::new(&gbm_device, logger.clone())?;
        let egl_context = EGLContext::new(&egl_display, logger.clone())?;

        // Store the supported formats
        let formats = egl_context
            .dmabuf_texture_formats()
            .iter()
            .map(|format| format.modifier)
            .collect::<Vec<_>>();

        let renderer = unsafe { Gles2Renderer::new(egl_context, logger.clone()) }?;

        handle.insert_source(backend, handle_backend_event)?;

        Ok(X11Backend {
            logger,
            handle: x_handle,
            outputs: vec![],
            formats,
            renderer,
            _egl_display: egl_display,
            gbm_device: Arc::new(Mutex::new(gbm_device)),
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
        let window = WindowBuilder::new()
            .title("Output 1 - Press Super + O to create a new output")
            .build(&self.handle)?;

        let surface = self
            .handle
            .create_surface(&window, self.gbm_device.clone(), self.formats.iter().copied())?;

        // Create the output
        let output = X11Output { window, surface };

        self.outputs.push(output);

        Ok(())
    }

    fn create_new_output(&mut self) {
        // Get output number
        let number = self.outputs.len() + 1;

        let window = WindowBuilder::new()
            .title(&format!("Output {}", number))
            .build(&self.handle)
            .expect("Failed to create new output");

        let surface = self
            .handle
            .create_surface(&window, self.gbm_device.clone(), self.formats.iter().copied())
            .expect("Failed to create new surface");

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
fn handle_backend_event(event: X11Event, _: &mut (), name_me: &mut NameMe) {
    match event {
        X11Event::CloseRequested { window_id } => {
            let backend = name_me.state.downcast_backend_mut::<X11Backend>().unwrap();

            // Destroy the output for the window that has been closed.
            // TODO: Wait does the window get closed actually?
            backend.outputs.retain(|output| output.window.id() != window_id);

            if backend.outputs.is_empty() {
                info!(
                    name_me.state.backend().logger(),
                    "Quitting because all outputs are destroyed"
                );
                name_me.state.continue_loop = false;
            }
        }

        X11Event::Input(event) => {
            if let InputEvent::Keyboard { ref event } = event {
                if event.key_code() == 24 // KEY_O
                    && event.state() == KeyState::Pressed
                {
                    name_me.state.backend_mut().create_new_output();
                }
            }

            name_me.state.handle_input(event);
        }

        // TODO
        X11Event::Resized {
            window_id: _,
            new_size: _,
        } => (),

        X11Event::Refresh { window_id } | X11Event::PresentCompleted { window_id } => {
            // TODO: Rendering with damage.

            let backend = name_me.state.downcast_backend_mut::<X11Backend>().unwrap();

            // Only try to present if the output exists. If the output does not exist, we have likely recidved a present completed
            // event from a destroyed window.
            if let Some(output) = backend
                .outputs
                .iter_mut()
                .find(|output| output.window.id() == window_id)
            {
                match output.surface.buffer() {
                    Ok((dmabuf, _age)) => {
                        let size = match output.surface.window() {
                            Some(window) => {
                                let size = window.as_ref().size();
                                (size.w as i32, size.h as i32).into()
                            }

                            None => {
                                panic!("Dead window?");
                            }
                        };

                        backend.renderer.bind(dmabuf).expect("TODO: Bind handling");

                        backend
                            .renderer
                            .render(size, Transform::_180, |_renderer, frame| {
                                // TODO: Call rendering functions
                                frame.clear([0.5, 0.75, 0.5, 1.0], &[Rectangle::from_loc_and_size((0, 0), size)])
                            })
                            .map(Result::unwrap)
                            .expect("Rendering error");

                        backend.renderer.unbind().expect("unbind");

                        // Mark the buffer as submitted to present
                        output.surface.submit().expect("Submit buffer");
                    }

                    Err(alloc) => {
                        panic!("Allocate on acquire, {}", alloc);
                    }
                }
            }
        }
    }
}
