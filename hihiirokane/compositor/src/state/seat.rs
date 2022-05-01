use smithay::{
    delegate_seat,
    wayland::seat::{SeatHandler, SeatState},
};

use super::Hihiirokane;

impl SeatHandler for Hihiirokane {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        todo!()
    }
}

delegate_seat!(Hihiirokane);
