use smithay::{
    reexports::wayland_server::{protocol::wl_surface::WlSurface, DispatchData},
    wayland::shell::wlr_layer::LayerShellRequest,
};

use super::Layer;

pub fn handle_layer_commit(_surface: &WlSurface, _layer: &mut Layer) {
    todo!()
}

pub(crate) fn handle_layer_shell_request(_request: LayerShellRequest, mut _ddata: DispatchData) {
    todo!()
}
