use std::os::unix::prelude::RawFd;

use smithay::{
    reexports::wayland_server::protocol::{wl_data_device_manager, wl_data_source, wl_surface},
    wayland::{
        data_device::{ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler},
        seat::{Seat, SeatHandler, SeatState},
    },
};

use crate::State;

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.protocols.seat
    }
}

impl DataDeviceHandler for State {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.protocols.data_device
    }
}

impl ClientDndGrabHandler for State {
    fn started(
        &mut self,
        _source: Option<wl_data_source::WlDataSource>,
        _icon: Option<wl_surface::WlSurface>,
        _seat: Seat<Self>,
    ) {
    }

    fn dropped(&mut self, _seat: Seat<Self>) {}
}

// Aerugo does not have server side dnd.
impl ServerDndGrabHandler for State {
    fn action(&mut self, _action: wl_data_device_manager::DndAction) {
        unreachable!()
    }

    fn dropped(&mut self) {
        unreachable!()
    }

    fn cancelled(&mut self) {
        unreachable!()
    }

    fn send(&mut self, _mime_type: String, _fd: RawFd) {
        unreachable!()
    }

    fn finished(&mut self) {
        unreachable!()
    }
}
