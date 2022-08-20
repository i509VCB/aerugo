use smithay::{
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1,
        wayland_server::{
            protocol::{wl_output, wl_seat},
            DisplayHandle,
        },
    },
    wayland::{
        shell::{
            wlr_layer::{Layer, LayerSurface, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{
                decoration::XdgDecorationHandler, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
                XdgShellState,
            },
        },
        Serial,
    },
};

use crate::State;

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.protocols.xdg_shell
    }

    fn new_toplevel(&mut self, _dh: &DisplayHandle, _surface: ToplevelSurface) {
        todo!()
    }

    fn new_popup(&mut self, _dh: &DisplayHandle, _surface: PopupSurface, _positioner: PositionerState) {
        todo!()
    }

    fn grab(&mut self, _dh: &DisplayHandle, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        todo!()
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, _dh: &DisplayHandle, _toplevel: ToplevelSurface) {
        todo!()
    }

    fn request_mode(
        &mut self,
        _dh: &DisplayHandle,
        _toplevel: ToplevelSurface,
        _mode: zxdg_toplevel_decoration_v1::Mode,
    ) {
        todo!()
    }

    fn unset_mode(&mut self, _dh: &DisplayHandle, _toplevel: ToplevelSurface) {
        todo!()
    }
}

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.protocols.layer_shell
    }

    fn new_layer_surface(
        &mut self,
        _dh: &DisplayHandle,
        _surface: LayerSurface,
        _output: Option<wl_output::WlOutput>,
        _layer: Layer,
        _namespace: String,
    ) {
        todo!()
    }
}
