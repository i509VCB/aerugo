use smithay::{
    delegate_seat,
    wayland::seat::{SeatHandler, SeatState},
};

use super::State;

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        todo!()
    }
}

delegate_seat!(State);
