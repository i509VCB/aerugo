use smithay::{
    delegate_compositor,
    reexports::wayland_server::{protocol::wl_surface, DisplayHandle},
    wayland::compositor::{CompositorHandler, CompositorState},
};

use super::Hihiirokane;

impl CompositorHandler for Hihiirokane {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.protocols.compositor
    }

    fn commit(&mut self, _dh: &mut DisplayHandle<'_>, _surface: &wl_surface::WlSurface) {
        todo!()
    }
}

delegate_compositor!(Hihiirokane);
