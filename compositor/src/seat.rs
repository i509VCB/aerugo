use smithay::input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState};
use wayland_server::protocol::wl_surface;

use crate::AerugoCompositor;

impl SeatHandler for AerugoCompositor {
    type KeyboardFocus = wl_surface::WlSurface;
    type PointerFocus = wl_surface::WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {}

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}
