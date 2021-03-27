use std::sync::Mutex;

use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    wayland::{
        compositor::with_states,
        shell::xdg::{ToplevelSurface, XdgToplevelSurfaceRoleAttributes},
    },
};

use super::Toplevel;

#[derive(Debug)]
pub enum ToplevelInner {
    Xdg(ToplevelSurface),
    #[cfg(feature = "xwayland")]
    X11(X11Surface),
}

impl ToplevelInner {
    pub fn alive(&self) -> bool {
        match self {
            ToplevelInner::Xdg(inner) => inner.alive(),
            #[cfg(feature = "xwayland")]
            ToplevelInner::X11(inner) => inner.alive(),
        }
    }

    pub fn get_surface(&self) -> Option<&WlSurface> {
        match self {
            ToplevelInner::Xdg(inner) => inner.get_surface(),
            #[cfg(feature = "xwayland")]
            ToplevelInner::X11(inner) => inner.get_surface(),
        }
    }

    pub fn send_configure(&self) {
        match self {
            ToplevelInner::Xdg(inner) => inner.send_configure(),
            #[cfg(feature = "xwayland")]
            ToplevelInner::X11(inner) => todo!(),
        }
    }
}

pub fn handle_toplevel_commit(surface: &WlSurface, toplevel: &mut Toplevel) {
    // Upon first commit, send the initial configure
    let initial_configure_sent = with_states(surface, |states| {
        states
            .data_map
            .get::<Mutex<XdgToplevelSurfaceRoleAttributes>>()
            .unwrap()
            .lock()
            .unwrap()
            .initial_configure_sent
    })
    .unwrap();

    if !initial_configure_sent {
        toplevel.send_configure();
    }

    // TODO: Refresh toplevel?
}
