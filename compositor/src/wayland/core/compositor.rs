use std::borrow::Cow;

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    wayland::compositor::{self, CompositorClientState, CompositorHandler, CompositorState},
};
use wayland_server::{protocol::wl_surface::WlSurface, Client};

use crate::{shell::Shell, state::ClientData, Aerugo};

impl CompositorHandler for Aerugo {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.wl_compositor
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Let Smithay perform buffer management for us.
        //
        // on_commit_buffer_handler will manage the buffer, damage and opaque regions.
        on_commit_buffer_handler::<Self>(surface);

        // If the surface is sync the parent needs to be committed to apply the pending state.
        //
        // The parent surface will always return `false`
        if compositor::is_sync_subsurface(surface) {
            return;
        }

        // Select the root surface if a desync subsurface was committed.
        let mut surface = Cow::Borrowed(surface);

        while let Some(parent) = compositor::get_parent(&surface) {
            surface = Cow::Owned(parent);
        }

        // Commit the root surface state in the shell. This will complete any transactions that are in flight
        // and are waiting for the acked state to be applied.
        Shell::commit(self, &surface);
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        ClientData::get_data(client).unwrap().client_compositor_state()
    }
}

smithay::delegate_compositor!(Aerugo);
