use smithay::{
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Serial},
    wayland::shell::xdg::{Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
};
use wayland_server::protocol::{wl_output, wl_seat, wl_surface};

use crate::{scene::NodeIndex, AerugoCompositor};

impl XdgShellHandler for AerugoCompositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // TODO: Remove this horrible temporary thing.
        surface.send_configure();
        let index = self.scene.create_surface_tree(surface.wl_surface().clone());
        self.scene.set_output_node(&self.output, NodeIndex::SurfaceTree(index));
        dbg!(index);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // TODO: track popup
    }

    fn move_request(&mut self, _surface: ToplevelSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // TODO: Forward to wm
    }

    fn resize_request(
        &mut self,
        _surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial,
        _edges: xdg_toplevel::ResizeEdge,
    ) {
        // TODO: forward to wm
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        // TODO
    }

    fn maximize_request(&mut self, _surface: ToplevelSurface) {
        // TODO: forward to wm
    }

    fn unmaximize_request(&mut self, _surface: ToplevelSurface) {
        // TODO: forward to wm
    }

    fn fullscreen_request(&mut self, _surface: ToplevelSurface, _output: Option<wl_output::WlOutput>) {
        // TODO: forward to wm
    }

    fn unfullscreen_request(&mut self, _surface: ToplevelSurface) {
        // TODO: forward to wm
    }

    fn minimize_request(&mut self, _surface: ToplevelSurface) {
        // TODO: forward to wm
    }

    fn show_window_menu(
        &mut self,
        _surface: ToplevelSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial,
        _location: Point<i32, Logical>,
    ) {
        // TODO: Forward to wm
    }

    fn ack_configure(&mut self, _surface: wl_surface::WlSurface, _configure: Configure) {
        // TODO: Notify wm about current window state
    }

    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {
        // TODO: forward to wm
    }

    fn toplevel_destroyed(&mut self, _surface: ToplevelSurface) {
        // TODO: Handle by destroying toplevel handles.
    }

    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        // TODO: Handle popup death
    }
}

smithay::delegate_xdg_shell!(AerugoCompositor);
