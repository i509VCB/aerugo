//! X11 input and output backend

use calloop::LoopHandle;
use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        egl::{EGLContext, EGLDisplay},
        renderer::{gles2::Gles2Renderer, Bind, Frame, Renderer},
        x11::{self, WindowBuilder, X11Handle, X11Surface},
    },
    reexports::gbm,
    utils::{DeviceFd, Rectangle},
    wayland::{
        dmabuf::{DmabufGlobal, DmabufState, ImportError},
        shm::ShmState,
    },
};
use wayland_server::DisplayHandle;

use crate::{
    cli::AerugoArgs,
    state::{Aerugo, AerugoCompositor},
};

#[derive(Debug)]
pub struct Backend {
    x11: X11Handle,
    window: x11::Window,
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
        let backend = x11::X11Backend::new(None).unwrap();
        let x11 = backend.handle();

        let window = WindowBuilder::new().title("Aerugo - X11").build(&x11).unwrap();
        window.map();
        let wid = window.id();

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

        r#loop
            .insert_source(backend, move |event, _, aerugo| match event {
                x11::X11Event::Refresh { window_id: _ } => {
                    let backend = aerugo.compositor().backend.downcast_mut::<Backend>().unwrap();
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
                x11::X11Event::Input(_) => {}
                x11::X11Event::Resized {
                    new_size: _,
                    window_id: _,
                } => {}
                x11::X11Event::PresentCompleted { window_id: _ } => {}
                x11::X11Event::CloseRequested { window_id } if window_id == wid => {
                    let backend = aerugo.compositor().backend.downcast_mut::<Backend>().unwrap();

                    backend.shutdown = true;
                    aerugo.check_shutdown();
                }
                _ => (),
            })
            .unwrap();

        Ok(Self {
            x11,
            window,
            r#loop: r#loop.clone(),
            display: display.clone(),
            // TODO: Renderer shm formats
            shm_state: ShmState::new::<AerugoCompositor, _>(display, Vec::with_capacity(2), None),
            shutdown: false,
            renderer,
            surface,
        })
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
