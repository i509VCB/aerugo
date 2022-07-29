use std::{io, os::unix::net::UnixStream, sync::Arc};

use smithay::{
    reexports::wayland_server::{
        backend::{ClientData, ClientId, DisconnectReason},
        Client, DisplayHandle,
    },
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        output::OutputManagerState,
        seat::{Seat, SeatState},
        shell::{wlr_layer::WlrLayerShellState, xdg::XdgShellState},
        shm::ShmState,
    },
};

/// The compositor state.
#[derive(Debug)]
pub struct State {
    display: DisplayHandle,
    protocols: Protocols,
    seat: Seat<Self>,
    // TODO: XWayland
}

/// Protocol state objects.
#[derive(Debug)]
pub struct Protocols {
    compositor: CompositorState,
    seat: SeatState<State>,
    data_device: DataDeviceState,
    output: OutputManagerState,
    xdg_shell: XdgShellState,
    layer_shell: WlrLayerShellState,
    shm: ShmState,
    // TODO: Dmabuf?
}

/// Data associated with a [`Client`].
pub struct ClientInfo;

impl ClientInfo {
    /// Returns information about the client.
    ///
    /// # Panics
    ///
    /// This function will panic if the client was not created using [`ClientInfo::create_client`].
    pub fn get_info(client: &Client) -> &Self {
        client
            .get_data()
            .expect("Failed to get ClientInfo, a client may have been externally created?")
    }

    /// Creates a client with the specified information.
    ///
    /// This function takes ownership of the `stream`.
    pub fn create_client(display: &mut DisplayHandle, stream: UnixStream, info: Self) -> io::Result<Client> {
        display.insert_client(stream, Arc::new(info))
    }
}

impl ClientData for ClientInfo {
    // TODO: Log connection events.
    fn initialized(&self, _client: ClientId) {}
    fn disconnected(&self, _client: ClientId, _reason: DisconnectReason) {}
}
