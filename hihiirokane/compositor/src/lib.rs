pub mod backend;
pub mod client;
pub mod output;
pub mod state;

pub mod format;
// pub mod vulkan;

use std::{error::Error, ffi::OsString, io, sync::Arc, time::Duration};

use smithay::{
    reexports::{
        calloop::{EventLoop, LoopHandle},
        wayland_server::Display,
    },
    wayland::socket::ListeningSocketSource,
};
use state::State;

use crate::client::DumbClientData;

#[derive(Debug)]
pub struct Hihiirokane {
    pub state: State,
    pub display: Display<State>,
}

impl Hihiirokane {
    // TODO: How to pass backends around?
    pub fn new(
        _loop_handle: &LoopHandle<'_, Hihiirokane>,
        mut display: Display<State>,
    ) -> Result<Hihiirokane, Box<dyn Error>> {
        Ok(Hihiirokane {
            state: State::new(&mut display),
            display,
        })
    }

    pub fn run(mut self, mut event_loop: EventLoop<Hihiirokane>) -> io::Result<()> {
        let signal = event_loop.get_signal();

        event_loop.run(Duration::from_millis(5), &mut self, |hihiirokane| {
            if !hihiirokane.running() {
                signal.stop();
            }

            // TODO: Poll source
            hihiirokane
                .display
                .dispatch_clients(&mut hihiirokane.state)
                .expect("dispatch");

            // TODO: Better io error handling?
            hihiirokane.display.flush_clients().expect("flush");
        })
    }

    pub fn create_socket(&mut self, loop_handle: &LoopHandle<'_, Hihiirokane>) -> Result<OsString, Box<dyn Error>> {
        let socket = ListeningSocketSource::with_name("wayland-15", None)?;
        println!("Using socket name {:?}", socket.socket_name());

        let socket_name = socket.socket_name().to_owned();

        loop_handle.insert_source(socket, |new_client, _, hihiirokane| {
            hihiirokane
                .display
                .insert_client(new_client, Arc::new(DumbClientData))
                .expect("handle error?");
        })?;

        Ok(socket_name)
    }

    pub fn running(&self) -> bool {
        self.state.running
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

    use crate::Hihiirokane;

    #[test]
    fn run_simple() {
        let event_loop = EventLoop::try_new().unwrap();
        let display = Display::new().unwrap();
        let loop_handle = event_loop.handle();

        let mut hihiirokane = Hihiirokane::new(&loop_handle, display).unwrap();
        let socket_name = hihiirokane.create_socket(&loop_handle).unwrap();

        // TODO: Better client spawning
        {
            Command::new("wayland-info")
                .env("WAYLAND_DISPLAY", &socket_name)
                .spawn()
                .expect("spawn");
        }

        hihiirokane.run(event_loop).unwrap();
    }
}
