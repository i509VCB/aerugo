pub mod client;
pub mod state;

// Misc stuff for upstream
pub mod format;
// pub mod vulkan;

use std::{error::Error, ffi::OsString, sync::Arc, time::Duration};

use smithay::{
    reexports::{
        calloop::{self, EventLoop, LoopHandle},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display,
        },
    },
    wayland::socket::ListeningSocketSource,
};
use state::Aerugo;

#[derive(Debug)]
pub struct AerugoCompositor {
    pub state: Aerugo,
    pub display: Display<Aerugo>,
}

impl AerugoCompositor {
    // TODO: How to pass backends around?
    pub fn new(
        _loop_handle: &LoopHandle<'_, AerugoCompositor>,
        display: Display<Aerugo>,
    ) -> Result<AerugoCompositor, Box<dyn Error>> {
        Ok(AerugoCompositor {
            state: Aerugo::new(&display.handle()),
            display,
        })
    }

    pub fn run(mut self, mut event_loop: EventLoop<AerugoCompositor>) -> calloop::Result<()> {
        let signal = event_loop.get_signal();

        event_loop.run(Duration::from_millis(5), &mut self, |aerugo| {
            if !aerugo.running() {
                signal.stop();
            }

            // TODO: Poll source
            aerugo.display.dispatch_clients(&mut aerugo.state).expect("dispatch");

            // TODO: Better io error handling?
            aerugo.display.flush_clients().expect("flush");
        })
    }

    pub fn create_socket(
        &mut self,
        loop_handle: &LoopHandle<'_, AerugoCompositor>,
    ) -> Result<OsString, Box<dyn Error>> {
        let socket = ListeningSocketSource::new_auto(None)?;
        println!("Using socket name {:?}", socket.socket_name());

        let socket_name = socket.socket_name().to_owned();

        loop_handle.insert_source(socket, |new_client, _, aerugo| {
            aerugo
                .display
                .handle()
                .insert_client(new_client, Arc::new(DumbClientData))
                .expect("handle error?");
        })?;

        Ok(socket_name)
    }

    pub fn running(&self) -> bool {
        self.state.running
    }
}

pub struct DumbClientData;

impl ClientData for DumbClientData {
    fn initialized(&self, _: ClientId) {}

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        println!("{:?}: {:?}", client_id, reason);
    }
}

#[cfg(test)]
mod tests {
    use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

    use crate::{client::SpawnClient, AerugoCompositor};

    #[test]
    fn run_simple() {
        let event_loop = EventLoop::try_new().unwrap();
        let display = Display::new().unwrap();
        let loop_handle = event_loop.handle();

        let mut aerugo = AerugoCompositor::new(&loop_handle, display).unwrap();
        let socket_name = aerugo.create_socket(&loop_handle).unwrap();

        // TODO: Better client spawning
        {
            SpawnClient::new("wayland-info")
                .wayland_display(&socket_name)
                .spawn()
                .expect("spawn");
        }

        aerugo.run(event_loop).unwrap();
    }
}
