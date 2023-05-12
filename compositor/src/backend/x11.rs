//! X11 input and output backend

use calloop::LoopHandle;
use smithay::{
    backend::{
        allocator::{
            dmabuf::{Dmabuf, DmabufAllocator},
            gbm::GbmAllocator,
        },
        egl::{EGLContext, EGLDisplay},
        renderer::{
            element::AsRenderElements, gles2::Gles2Renderer, utils::draw_render_elements, Bind, Frame, Renderer,
        },
        x11::{Window, WindowBuilder, X11Backend, X11Event, X11Handle, X11Surface},
    },
    reexports::gbm::{self, BufferObjectFlags},
    utils::{DeviceFd, Rectangle, Transform},
    wayland::{
        dmabuf::{DmabufGlobal, DmabufState, ImportError},
        shm::ShmState,
    },
};
use wayland_server::DisplayHandle;

use crate::{scene::SceneGraphElement, Aerugo, Loop};

#[derive(Debug)]
pub struct Backend {
    x11: X11Handle,
    window: Window,
    renderer: Gles2Renderer,
    surface: X11Surface,
    r#loop: LoopHandle<'static, Loop>,
    display: DisplayHandle,
    shm_state: ShmState,
    shutdown: bool,
}

impl dyn super::Backend {
    fn x11_mut(&mut self) -> &mut Backend {
        self.downcast_mut().expect("Not X11")
    }
}

impl Backend {
    // TODO: Error type
    pub fn new(r#loop: LoopHandle<'static, Loop>, display: DisplayHandle) -> Result<Self, ()> {
        let backend = X11Backend::new().unwrap();
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
        let egl = EGLDisplay::new(device.clone()).unwrap();
        let context = EGLContext::new(&egl).unwrap();

        let surface = x11
            .create_surface(
                &window,
                DmabufAllocator(GbmAllocator::new(device.clone(), BufferObjectFlags::RENDERING)),
                context.dmabuf_render_formats().iter().map(|format| format.modifier),
            )
            .unwrap();

        let renderer = unsafe { Gles2Renderer::new(context) }.unwrap();

        r#loop.insert_source(backend, dispatch_x11_event).unwrap();

        Ok(Self {
            x11,
            window,
            r#loop,
            display: display.clone(),
            // TODO: Additional renderer shm formats
            shm_state: ShmState::new::<Aerugo>(&display, Vec::with_capacity(2)),
            shutdown: false,
            renderer,
            surface,
        })
    }
}

fn dispatch_x11_event(event: X11Event, _: &mut (), aerugo: &mut Loop) {
    match event {
        X11Event::Refresh { window_id: _ } => draw(aerugo),
        X11Event::Input(_) => {}
        X11Event::Resized {
            new_size: _,
            window_id: _,
        } => draw(aerugo),
        X11Event::PresentCompleted { window_id: _ } => draw(aerugo),
        X11Event::CloseRequested { window_id: _ } => {
            // TODO: shutdown based on output counts
            let backend: &mut Backend = &mut aerugo.comp.backend.downcast_mut().unwrap();
            backend.shutdown = true;
            aerugo.check_shutdown();
        }
    }
}

fn draw(aerugo: &mut Loop) {
    let backend = aerugo.comp.backend.x11_mut();
    let (buffer, _age) = backend.surface.buffer().unwrap();
    backend.renderer.bind(buffer).unwrap();

    let elems: Vec<SceneGraphElement> = if let Some(hir) = aerugo.comp.scene.get_graph(&aerugo.comp.output) {
        hir.render_elements(
            &mut backend.renderer,
            (0, 0).into(),
            smithay::utils::Scale { x: 1., y: 1. },
        )
        .into()
    } else {
        Vec::new()
    };

    {
        let mut frame = backend
            .renderer
            .render(
                (backend.window.size().w as i32, backend.window.size().h as i32).into(),
                Transform::Normal,
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

        draw_render_elements::<Gles2Renderer, _, _>(
            &mut frame,
            1.0,
            &elems,
            &[Rectangle::from_loc_and_size((0, 0), (i32::MAX, i32::MAX))],
        )
        .unwrap();

        frame.finish().unwrap();
    }

    backend.surface.submit().unwrap();
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
