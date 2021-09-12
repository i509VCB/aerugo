use std::sync::Mutex;

use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{compositor::with_states, shell::xdg::XdgPopupSurfaceRoleAttributes},
};

use super::Popup;

pub fn handle_popup_commit(surface: &WlSurface, popup: &mut Popup) {
    // Upon first commit, send the initial configure
    let initial_configure_sent = with_states(surface, |states| {
        states
            .data_map
            .get::<Mutex<XdgPopupSurfaceRoleAttributes>>()
            .unwrap()
            .lock()
            .unwrap()
            .initial_configure_sent
    })
    .unwrap();

    if !initial_configure_sent {
        // The initial configure should never fail.
        popup.send_configure().unwrap();
    }
}
