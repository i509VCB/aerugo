//! X11 input and output backend

use calloop::LoopHandle;
use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::{EGLContext, EGLDisplay},
        renderer::{gles2::Gles2Renderer, Bind, Frame, Renderer},
        x11::{Window, WindowBuilder, X11Backend, X11Event, X11Handle, X11Surface},
    },
    reexports::gbm,
    utils::{DeviceFd, Rectangle},
    wayland::{
        dmabuf::{DmabufGlobal, DmabufState, ImportError},
        shm::ShmState,
    },
};
use wayland_server::DisplayHandle;

use crate::{cli::AerugoArgs, Aerugo, AerugoCompositor};

#[derive(Debug)]
pub struct Backend {
    x11: X11Handle,
    window: Window,
    renderer: Gles2Renderer,
    surface: X11Surface,
    r#loop: LoopHandle<'static, Aerugo>,
    display: DisplayHandle,
    shm_state: ShmState,
    shutdown: bool,
}

impl Backend {
    // TODO: Error type
    pub fn new(r#loop: &LoopHandle<'static, Aerugo>, display: &DisplayHandle, _args: &AerugoArgs) -> Result<Self, ()> {
        let backend = X11Backend::new(None).unwrap();
        let x11 = backend.handle();

        // TODO: Initialize output with window.
        //
        // TODO for Smithay:
        // - Allow specifying the format of the buffers presented to the window. For now we rely on the X11
        //   backend to select Argb8888 or Xrgb8888. It may be desireable however to use Argb2101010 if
        //   available. This will however require a way to enumerate what formats the window could be created
        //   with.
        let window = WindowBuilder::new().title("Aerugo").build(&x11).unwrap();
        window.map();

        // Get the drm node for buffer allocation and initializing EGL.
        //
        // TODO for Smithay:
        // - This should return just the path to the drm device. For the legacy DRI3 fallback, there should be
        //   a separate function to get the DRM file descriptor in that case.
        let (_, fd) = x11.drm_node().expect("Failed to get DRM node used by X server");
        let device = gbm::Device::new(DeviceFd::from(fd)).unwrap();
        let egl = EGLDisplay::new(device.clone(), None).unwrap();
        let context = EGLContext::new(&egl, None).unwrap();

        let surface = x11
            .create_surface(
                &window,
                device,
                context.dmabuf_render_formats().iter().map(|format| format.modifier),
            )
            .unwrap();

        let renderer = unsafe { Gles2Renderer::new(context, None) }.unwrap();

        r#loop.insert_source(backend, dispatch_x11_event).unwrap();

        Ok(Self {
            x11,
            window,
            r#loop: r#loop.clone(),
            display: display.clone(),
            // TODO: Additional renderer shm formats
            shm_state: ShmState::new::<AerugoCompositor, _>(display, Vec::with_capacity(2), None),
            shutdown: false,
            renderer,
            surface,
        })
    }
}

fn dispatch_x11_event(event: X11Event, _: &mut (), aerugo: &mut Aerugo) {
    fn get_backend(compositor: &mut AerugoCompositor) -> &mut Backend {
        compositor.backend.downcast_mut().unwrap()
    }

    match event {
        X11Event::Refresh { window_id: _ } => {
            let backend = get_backend(&mut aerugo.comp);
            let (buffer, _age) = backend.surface.buffer().unwrap();
            backend.renderer.bind(buffer).unwrap();

            {
                let mut frame = backend
                    .renderer
                    .render(
                        (backend.window.size().w as i32, backend.window.size().h as i32).into(),
                        smithay::utils::Transform::Normal,
                    )
                    .unwrap();

                frame
                    .clear(
                        [0.8, 0.8, 0.8, 1.0],
                        &[Rectangle::from_loc_and_size(
                            (0, 0),
                            (backend.window.size().w as i32, backend.window.size().h as i32),
                        )],
                    )
                    .unwrap();

                // TODO: Actual rendering lol

                frame.finish().unwrap();
            }

            backend.surface.submit().unwrap();
        }
        X11Event::Input(_) => {}
        X11Event::Resized {
            new_size: _,
            window_id: _,
        } => {}
        X11Event::PresentCompleted { window_id: _ } => {}
        X11Event::CloseRequested { window_id: _ } => {
            // TODO: shutdown based on output counts
            let backend = get_backend(&mut aerugo.comp);
            backend.shutdown = true;
            aerugo.check_shutdown();
        }
    }
}

impl crate::backend::Backend for Backend {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }

    fn dmabuf_state(&mut self) -> &mut DmabufState {
        todo!("X11 does not initialize the dmabuf global yet")
    }

    fn dmabuf_imported(&mut self, _global: &DmabufGlobal, _dmabuf: Dmabuf) -> Result<(), ImportError> {
        todo!("X11 does not initialize the dmabuf global yet")
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown
    }
}
