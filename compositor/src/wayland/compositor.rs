use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    wayland::compositor::{CompositorHandler, CompositorState},
};
use wayland_server::protocol::wl_surface;

use crate::AerugoCompositor;

impl CompositorHandler for AerugoCompositor {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.wl_compositor
    }

    fn commit(&mut self, surface: &wl_surface::WlSurface) {
        // Let smithay take over buffer handling
        on_commit_buffer_handler(surface);

        // Allow smithay to import buffers for us

        //self.scene.commit(surface);
    }
}

smithay::delegate_compositor!(AerugoCompositor);
