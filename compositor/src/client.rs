use std::{io, os::unix::net::UnixStream, sync::Arc};

use smithay::reexports::wayland_server::{
    backend::{self, ClientId, DisconnectReason},
    Client, DisplayHandle,
};

/// Data associated with a [`Client`].
pub struct ClientData;

impl ClientData {
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

impl backend::ClientData for ClientData {
    // TODO: Log connection events.
    fn initialized(&self, _client: ClientId) {}
    fn disconnected(&self, _client: ClientId, _reason: DisconnectReason) {}
}
