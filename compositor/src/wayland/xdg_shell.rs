use smithay::{
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Serial},
    wayland::shell::xdg::{
        Configure, PopupSurface, PositionerState, ShellClient, ToplevelSurface, XdgShellHandler, XdgShellState,
    },
};
use wayland_server::protocol::{wl_output, wl_seat, wl_surface};

use crate::Aerugo;

impl XdgShellHandler for Aerugo {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell
    }

    fn new_client(&mut self, _client: ShellClient) {}

    fn client_pong(&mut self, _client: ShellClient) {}

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        self.shell.pending_toplevels.push(surface);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // TODO: track popups
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

smithay::delegate_xdg_shell!(Aerugo);
